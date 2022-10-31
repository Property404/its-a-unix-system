#pragma once
#include <memory>
#include <ostream>

// Should act as 'cout', but displays to web terminal instead of
// console.log()
extern std::shared_ptr<std::ostream> jout;
// Web terminal analog to cerr
extern std::shared_ptr<std::ostream> jerr;
