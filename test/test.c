#include<stdio.h>
#include<string.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>
//gcc test.c -fno-stack-protector -z execstack
unsigned char code[] = \
"\x31\xc0\x50\x68\x6e\x2f\x73\x68\x68\x2f\x2f\x62\x69\x89\xe3\x50\x89\xe2\x53\x89\xe1\xb0\x0b\xcd\x80";

typedef struct Data{
  char *buff;
  int size;
}data_t;

data_t get_shellcode(){
FILE *f = fopen("../text.data", "rb");
fseek(f, 0, SEEK_END);
long fsize = ftell(f);
fseek(f, 0, SEEK_SET);  /* same as rewind(f); */

char *bytes = malloc(fsize + 1);
fread(bytes, 1, fsize, f);
fclose(f);
printf("Shellcode length: %d\n", fsize);
data_t data = {bytes, fsize};
return data;
}

// Make address range executable
int mprotect_executable(char *addr, int size) {
  return mprotect((void *)((long int)addr & ~(getpagesize() - 1)), size, PROT_READ | PROT_WRITE | PROT_EXEC);
}

// get page size
int getpagesize() {
  return sysconf(_SC_PAGESIZE);
}

char *hello = "fuck you";
int printlol(int a, int b){
  printf("A: %d B: %d\n", a, b);
  printf("%s\n", hello);
  return 3;
}

main()
{
  printf("hello: %p\n", hello);
  printf("func: %p\n", printlol);
  data_t data = get_shellcode();
  long int r =  mprotect_executable(data.buff, data.size); //mprotect((void *)((long int)code & ~4095),  4096, PROT_READ | PROT_WRITE|PROT_EXEC);
  mprotect_executable(hello, strlen(hello));
  printf("mprotect: %d\n", r);
  int (*ret)() = (int(*)())data.buff;
  return ret();

}
