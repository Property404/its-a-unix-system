#pragma once
#include <optional>
#include <string>
#include <vector>

#include "shell.h"

std::optional<int> execute_command(
    const Shell& shell, const std::string& command,
    const std::vector<std::string>& arguments);
