/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Offline native C consumer. This program does not initialize/connect Bluetooth. */
#include "openbuds.h"
#include <stdio.h>
#include <string.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr,"ABI check failed at line %d: %s\n",__LINE__,#x); return 1; } } while (0)
_Static_assert(sizeof(ob_air7_noise_v1)==4,"noise POD layout");
_Static_assert(sizeof(ob_error_info_v1)==16,"error POD layout");
typedef struct mock_stream {
 uint8_t incoming[128],outgoing[128];
 size_t available,position,used;
 unsigned reads,writes,releases;
 ob_handle_v1 handle;
 int reentry_checked;
} mock_stream;
static ob_status_v1 mock_read(void *context,uint8_t *dst,size_t capacity,uint32_t timeout_ms,size_t *read_count) {
 mock_stream *m=(mock_stream*)context; size_t n;
 m->reads++;
 if (timeout_ms==0) return OB_IO;
 /* Reentry must fail promptly, never deadlock a callback. */
 if (!m->reentry_checked) { ob_air7_noise_v1 noise={0};
  if (ob_v1_air7_get_noise(m->handle,&noise)!=OB_BUSY) return OB_IO;
  m->reentry_checked=1;
 }
 if (m->position==m->available) return OB_TIMEOUT;
 n=m->available-m->position; if(n>capacity)n=capacity;if(n>3)n=3;
 memcpy(dst,m->incoming+m->position,n);m->position+=n;*read_count=n;return OB_OK;
}
static ob_status_v1 mock_write(void *context,const uint8_t *src,size_t length,size_t *write_count) {
 mock_stream *m=(mock_stream*)context;size_t n=length>2?2:length;
 m->writes++;if(m->used+n>sizeof m->outgoing)return OB_IO;
 memcpy(m->outgoing+m->used,src,n);m->used+=n;*write_count=n;
 if(m->used>=2 && m->used==(size_t)m->outgoing[1]+2){
  const uint8_t *q=m->outgoing;uint16_t command=(uint16_t)(q[4]|((uint16_t)q[5]<<8));
  uint8_t payload[16]={0};size_t count=0;
  if(command==0x0100){const uint8_t caps[]={0,0xbf,0x0f,0x5a,0xe6,0x65,0x9c,0x40};memcpy(payload,caps,sizeof caps);count=sizeof caps;}
  else if(command==0x0101){payload[0]=0;payload[1]=0xc0;payload[2]=8;count=3;}
  else if(command==0x010c){payload[0]=0;payload[1]=1;payload[2]=1;payload[3]=8;count=4;}
  else return OB_IO;
  m->incoming[0]=0xaa;m->incoming[1]=(uint8_t)(7+count);m->incoming[2]=0;m->incoming[3]=0;
  m->incoming[4]=q[4];m->incoming[5]=(uint8_t)(q[5]|0x80);m->incoming[6]=q[6];m->incoming[7]=(uint8_t)count;m->incoming[8]=0;
  memcpy(m->incoming+9,payload,count);m->available=9+count;m->position=0;m->used=0;
 }
 return OB_OK;
}
static void mock_release(void *context){((mock_stream*)context)->releases++;}
int main(void){
 mock_stream m={0};ob_transport_v1 t={0};ob_handle_v1 h=0;ob_buffer_v1 b={0};ob_air7_noise_v1 noise={0};ob_error_info_v1 error={0};
 const char model[]=OB_DRIVER_AIR7,request[]="{\"op\":\"noise\"}";
 t.struct_size=(uint32_t)sizeof t;t.abi_version=OB_ABI_V1;t.context=&m;t.read=mock_read;t.write=mock_write;t.release=mock_release;
 CHECK(ob_v1_abi_version()==OB_ABI_V1);
 CHECK(ob_v1_open((const uint8_t*)"unknown",7,&t,&h)==OB_UNKNOWN_DRIVER);CHECK(h==0 && m.releases==0 && m.writes==0);
 CHECK(ob_v1_open((const uint8_t*)model,strlen(model),&t,&h)==OB_OK);m.handle=h;
 CHECK(m.writes==0);CHECK(ob_v1_air7_get_noise(h,&noise)==OB_NOT_INITIALIZED);
 CHECK(ob_v1_set_timeout(h,1000)==OB_OK);CHECK(ob_v1_initialize(h)==OB_OK);
 CHECK(ob_v1_air7_get_noise(h,&noise)==OB_OK);CHECK(noise.action==1 && noise.kind==1 && noise.value==OB_AIR7_MAXIMUM);
 CHECK(ob_v1_call(h,(const uint8_t*)request,strlen(request),&b)==OB_OK);
 CHECK(b.id!=0 && b.len>0);CHECK(memchr(b.data,'A',b.len)!=NULL);
 CHECK(ob_v1_buffer_release(&b)==OB_OK);CHECK(b.id==0 && b.data==NULL && b.len==0);CHECK(ob_v1_buffer_release(&b)==OB_OK);
 CHECK(ob_v1_air7_set_noise(h,999)==OB_INVALID_ARGUMENT);
 CHECK(ob_v1_last_error(&error,&b)==OB_OK);CHECK(error.status==OB_INVALID_ARGUMENT && b.len>0);CHECK(ob_v1_buffer_release(&b)==OB_OK);
 CHECK(ob_v1_call(h,NULL,1,&b)==OB_INVALID_ARGUMENT);CHECK(b.id==0);
 CHECK(ob_v1_close(h)==OB_OK);CHECK(m.releases==1);
 CHECK(ob_v1_close(h)==OB_INVALID_HANDLE);CHECK(m.releases==1);
 CHECK(m.reentry_checked && m.reads>0 && m.writes>0);
 puts("C ABI smoke passed: callback transport, partial I/O, ownership, reentry, JSON and POD API");return 0;
}
