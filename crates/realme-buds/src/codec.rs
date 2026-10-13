// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! AA + unsigned LEB128 length + control + reserved + inner packet.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

pub const MAX_PAYLOAD: usize = u16::MAX as usize;
const MAX_BUFFER: usize = (MAX_PAYLOAD + 32) * 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packet {
    pub command: u16,
    pub transaction: u8,
    pub payload: Vec<u8>,
}
impl Packet {
    pub fn new(command: u16, transaction: u8, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            command,
            transaction,
            payload: payload.into(),
        }
    }
    pub fn inner(&self) -> Result<Vec<u8>> {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(Error::Invalid("payload too large"));
        }
        let mut p = Vec::with_capacity(5 + self.payload.len());
        p.extend(self.command.to_le_bytes());
        p.push(self.transaction);
        p.extend((self.payload.len() as u16).to_le_bytes());
        p.extend(&self.payload);
        Ok(p)
    }
    pub fn parse(p: &[u8]) -> Result<Self> {
        if p.len() < 5 {
            return Err(Error::Protocol("short packet"));
        }
        let len = u16::from_le_bytes([p[3], p[4]]) as usize;
        let command = u16::from_le_bytes([p[0], p[1]]);
        // Air7 firmware 1.1.0.62: 0x841b declares one byte but appends opaque metadata 01.
        // The outer frame length is correct. Scope this exception to that exact shape.
        let air7_dynamic_ack =
            command == 0x841b && len == 1 && p.len() == 7 && p[5] == 0 && p[6] == 1;
        // Two additional read-reply quirks, captured on the same firmware:
        // language uses 0x0100 as its declared length; feature-status omits status/count.
        let air7_language = command == 0x8120 && len == 0x0100 && p.len() == 7 && p[5] == 0;
        let air7_status = command == 0x8129
            && p.len() >= 7
            && p[5] == 0
            && len == usize::from(p[6]) * 5
            && p.len() == len + 7;
        let air7_detect = command == 0x812a && len == 5 && p.len() == 11 && p[5] == 0;
        if p.len() != len + 5 && !air7_dynamic_ack && !air7_language && !air7_status && !air7_detect
        {
            return Err(Error::Protocol("inner length mismatch"));
        }
        Ok(Self::new(command, p[2], p[5..].to_vec()))
    }
    pub fn response_to(&self, request: &Self) -> bool {
        self.command == request.command | 0x8000 && self.transaction == request.transaction
    }
}
fn varint(mut n: usize) -> Vec<u8> {
    let mut b = Vec::new();
    loop {
        let v = (n & 127) as u8;
        n >>= 7;
        b.push(v | if n != 0 { 128 } else { 0 });
        if n == 0 {
            return b;
        }
    }
}
fn frame(control: u8, fragment: Option<u8>, data: &[u8]) -> Vec<u8> {
    let n = data.len() + 2 + usize::from(fragment.is_some());
    let mut out = vec![0xaa];
    out.extend(varint(n));
    out.extend([control, 0]);
    if let Some(id) = fragment {
        out.push(id);
    }
    out.extend(data);
    out
}
/// Wrap a packet, splitting it into sequenced frames no larger than max_frame.
pub fn encode(packet: &Packet, max_frame: usize) -> Result<Vec<Vec<u8>>> {
    if !(16..=MAX_PAYLOAD).contains(&max_frame) {
        return Err(Error::Invalid("invalid frame limit"));
    }
    let p = packet.inner()?;
    let single = frame(0, None, &p);
    if single.len() <= max_frame {
        return Ok(vec![single]);
    }
    let chunk = max_frame - 7; // safe for three length octets + SOF + control + reserved + FSN
    if p.len().div_ceil(chunk) > 256 {
        return Err(Error::Invalid("too many fragments"));
    }
    let chunks: Vec<_> = p.chunks(chunk).collect();
    Ok(chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            frame(
                if i == 0 {
                    1
                } else if i + 1 == chunks.len() {
                    3
                } else {
                    2
                },
                Some(i as u8),
                c,
            )
        })
        .collect())
}

#[derive(Default)]
pub struct Decoder {
    buffer: Vec<u8>,
    partial: Option<(u8, Vec<u8>)>,
}
impl Decoder {
    /// Accept arbitrary stream chunks. Incomplete headers are retained, never indexed blindly.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Packet>> {
        if self.buffer.len() + bytes.len() > MAX_BUFFER {
            self.reset();
            return Err(Error::Protocol("buffer limit"));
        }
        self.buffer.extend(bytes);
        let mut out = Vec::new();
        loop {
            if self.buffer.is_empty() {
                break;
            }
            if self.buffer[0] != 0xaa {
                self.reset();
                return Err(Error::Protocol("missing frame marker"));
            }
            let mut length = 0;
            let mut end = None;
            for j in 1..=3 {
                let Some(&b) = self.buffer.get(j) else {
                    break;
                };
                length |= ((b & 127) as usize) << (7 * (j - 1));
                if b & 128 == 0 {
                    end = Some(j + 1);
                    break;
                }
                if j == 3 {
                    self.reset();
                    return Err(Error::Protocol("overlong frame length"));
                }
            }
            let Some(start) = end else {
                break;
            };
            if !(2..=MAX_PAYLOAD + 16).contains(&length) {
                self.reset();
                return Err(Error::Protocol("invalid frame length"));
            }
            if self.buffer.len() < start + length {
                break;
            }
            let f: Vec<_> = self.buffer.drain(..start + length).collect();
            let fsn = f[start] & 3;
            let data = &f[start + 2..];
            match fsn {
                0 => {
                    if self.partial.take().is_some() {
                        return Err(Error::Protocol("interrupted fragment sequence"));
                    }
                    out.push(Packet::parse(data)?);
                }
                1 => {
                    if data.first() != Some(&0) || self.partial.is_some() {
                        self.partial = None;
                        return Err(Error::Protocol("invalid first fragment"));
                    }
                    self.partial = Some((0, data[1..].to_vec()));
                }
                2 | 3 => {
                    let Some((&id, body)) = data.split_first() else {
                        return Err(Error::Protocol("empty fragment"));
                    };
                    let Some((last, mut p)) = self.partial.take() else {
                        return Err(Error::Protocol("orphan fragment"));
                    };
                    if last.checked_add(1) != Some(id) || p.len() + body.len() > MAX_PAYLOAD + 5 {
                        return Err(Error::Protocol("fragment sequence/size"));
                    }
                    p.extend(body);
                    if fsn == 3 {
                        out.push(Packet::parse(&p)?);
                    } else {
                        self.partial = Some((id, p));
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(out)
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.partial = None;
    }
}
