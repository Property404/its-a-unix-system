#pragma once
#include <optional>
#include <string>
#include <vector>

std::optional<int> execute_command(
    const std::string& command, const std::vector<std::string>& arguments);
