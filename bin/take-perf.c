#include <libgen.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char const **argv) {
  int user_uid = getuid();
  int user_gid = getgid();
  setuid(geteuid());
  setgid(getegid());

  char path[1024];
  int len = readlink("/proc/self/exe", path, sizeof(path) - 1);
  if (len == -1) {
    fprintf(stderr, "Failed to readlink /proc/self/exe\n");
    abort();
  }
  path[len] = '\0';

  char *dir = dirname(path);
  chdir(dir);
  chdir("..");

  unlink("perf.data.old");
  unlink("perf.data");
  system("perf record -F 999 -g --call-graph dwarf -- timeout 20 "
         "./target/release/lambda");
  chown("perf.data", user_uid, user_gid);

  return 0;
}