#include "commands.h"

#include <exception>
#include <filesystem>
#include <iostream>

#include "js_bindings.h"

static int echo(const std::vector<std::string>& args) {
    for (auto it = args.begin() + 1; it != args.end(); it++) {
        if (it != args.begin() + 1) {
            jout << " ";
        }
        jout << *it;
    }
    jout << std::endl;
    return 0;
}

static int pwd(const std::vector<std::string>& args) {
    (void)args;
    jout << std::filesystem::current_path().c_str() << std::endl;
    return 0;
}

static int ls(const std::vector<std::string>& args) {
    auto path = std::filesystem::current_path();
    if (args.size() >= 2) {
        path = args[1];
    }
    for (const auto& entry : std::filesystem::directory_iterator(path)) {
        jout << entry.path().c_str() << std::endl;
    }
    return 0;
}

static int cd(const std::vector<std::string>& args) {
    if (args.size() >= 2) {
        std::filesystem::current_path(args[1]);
    } else {
        std::filesystem::current_path("/");
    }
    return 0;
}

std::optional<int> execute_command(
    const std::string& command,
    const std::vector<std::string>& arguments) {
    try {
        if (command == "echo") {
            return echo(arguments);
        } else if (command == "pwd") {
            return pwd(arguments);
        } else if (command == "ls") {
            return ls(arguments);
        } else if (command == "cd") {
            return cd(arguments);
        }
    } catch (std::exception& e) {
        jerr << e.what() << std::endl;
        return 1;
    }
    return {};
}
