#include "commands.h"

#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <sstream>
#include <stdexcept>

static int echo(const Shell& shell, const std::vector<std::string>& args) {
    for (auto it = args.begin() + 1; it != args.end(); it++) {
        if (it != args.begin() + 1) {
            *(shell.out) << " ";
        }
        *(shell.out) << *it;
    }
    *(shell.out) << std::endl;
    return 0;
}

static int pwd(const Shell& shell, const std::vector<std::string>& args) {
    (void)args;
    *(shell.out) << std::filesystem::current_path().c_str() << std::endl;
    return 0;
}

static int ls(const Shell& shell, const std::vector<std::string>& args) {
    auto path = std::filesystem::current_path();
    if (args.size() >= 2) {
        path = args[1];
    }
    for (const auto& entry : std::filesystem::directory_iterator(path)) {
        *(shell.out) << entry.path().c_str() << std::endl;
    }
    return 0;
}

static int cd(const Shell& shell, const std::vector<std::string>& args) {
    (void)shell;
    if (args.size() >= 2) {
        std::filesystem::current_path(args[1]);
    } else {
        std::filesystem::current_path("/");
    }
    return 0;
}

static int cat(const Shell& shell, const std::vector<std::string>& args) {
    for (auto it = args.begin() + 1; it != args.end(); it++) {
        std::ifstream fp(*it);
        std::string contents;
        std::stringstream buffer;
        if (fp.fail()) {
            throw std::runtime_error("Could not open file " + *it);
        }
        buffer << fp.rdbuf();
        contents = buffer.str();
        fp.close();
        std::string new_contents = "";
        for (const char& c : contents) {
            if (c != '\r') new_contents += c;
        }
        *(shell.out) << new_contents;
    }
    return 0;
}

std::optional<int> execute_command(
    const Shell& shell, const std::string& command,
    const std::vector<std::string>& arguments) {
    try {
        if (command == "echo") {
            return echo(shell, arguments);
        } else if (command == "pwd") {
            return pwd(shell, arguments);
        } else if (command == "ls") {
            return ls(shell, arguments);
        } else if (command == "cd") {
            return cd(shell, arguments);
        } else if (command == "cat") {
            return cat(shell, arguments);
        }
    } catch (std::exception& e) {
        *(shell.err) << e.what() << std::endl;
        return 1;
    }
    return {};
}
