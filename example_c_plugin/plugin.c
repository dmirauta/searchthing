#include <stdio.h>

#define Str char *
#define MatchHandle int

Str entries[3] = {"apple", "banana", "coconut"};
MatchHandle dummy_match[4] = {0, 1, 2, -1};

Str init() { return "C plugin,text-x-objsrc"; }

MatchHandle *queery() { return dummy_match; }

void handle_selection(MatchHandle mh) {
  printf("C lib plugin handling: %s\n", entries[mh]);
}

Str name(MatchHandle mh) { return entries[mh]; }
Str desc(MatchHandle mh) { return entries[mh]; }
Str icon_name(MatchHandle mh) { return entries[mh]; }
