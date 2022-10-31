#pragma once
#include <memory>
#include <ostream>
#include <string>
#include <vector>

class Shell {
   public:
    std::shared_ptr<std::ostream> out;
    std::shared_ptr<std::ostream> err;

    Shell(std::shared_ptr<std::ostream> out,
          std::shared_ptr<std::ostream> err) {
        this->out = out;
        this->err = err;
    }

    int run(std::string&&);
};
