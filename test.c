#include <stdio.h>
#include <stdlib.h>

int main() {
    volatile float f = strtof("8.8817847263968443574e-16", NULL);
    printf("%a\n", f);
    volatile double d = strtod("8.8817847263968443574e-16", NULL);
    printf("%a\n", d);
    volatile float df = d;
    printf("%a\n", (float)df);
    return 0;
}
