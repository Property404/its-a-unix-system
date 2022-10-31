#include <emscripten.h>

#include <cstdio>
#include <iostream>
#include <string>

#include "js_bindings.h"
#include "shell.h"

void prompt() { *jout << "$ "; }

extern "C" EMSCRIPTEN_KEEPALIVE int process_line(const char* line) {
    Shell shell(jout, jerr);
    auto result = shell.run(std::string(line));
    prompt();
    return result;
}

int main() {
    *jout << "Welcome to this stupid project" << std::endl;
    prompt();
    return 0;
}
