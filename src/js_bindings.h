#pragma once
#include <ostream>

// Should act as 'cout', but displays to web terminal instead of
// console.log()
extern std::ostream jout;
// Web terminal analog to cerr
extern std::ostream jerr;
