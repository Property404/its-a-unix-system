#include "shell.h"

#include <filesystem>
#include <fstream>
#include <memory>
#include <stack>
#include <stdexcept>
#include <variant>

#include "commands.h"

enum class QuoteType { None, Single, Double };

namespace TokenType {
struct Value {
    std::string value;
    Value(std::string&& value) : value(std::move(value)){};
    Value(const std::string& value) : value(value){};
};

struct Pipe {};

struct FileRedirectOut {};

struct Command {
    std::vector<std::string> arguments;
};

struct Container {};
}  // namespace TokenType

using Token = std::variant<TokenType::Value, TokenType::Pipe,
                           TokenType::FileRedirectOut, TokenType::Command,
                           TokenType::Container>;

struct AstToken {
    Token type;
    std::vector<AstToken> children;
    AstToken(Token&& type) : type(std::move(type)){};
};

std::vector<Token> tokenize(std::string&& source) {
    std::vector<Token> tokens;
    std::string buffer = "";
    QuoteType quote = QuoteType::None;
    for (const char c : source) {
        switch (quote) {
        case QuoteType::None:
            if (c == '\'') {
                quote = QuoteType::Single;
                continue;
            }
            if (c == '"') {
                quote = QuoteType::Double;
                continue;
            }
            if (c == ' ' || c == '\t' || c == '\n' || c == '|' ||
                c == '>') {
                if (!buffer.empty()) {
                    tokens.push_back(TokenType::Value(buffer));
                    buffer = "";
                }
                if (c == '|') {
                    tokens.push_back(TokenType::Pipe{});
                } else if (c == '>') {
                    tokens.push_back(TokenType::FileRedirectOut{});
                }
                continue;
            }
            break;
        case QuoteType::Single:
            if (c == '\'') {
                quote = QuoteType::None;
                tokens.push_back(TokenType::Value(buffer));
                buffer = "";
                continue;
            }
            break;
        case QuoteType::Double:
            if (c == '"') {
                quote = QuoteType::None;
                tokens.push_back(TokenType::Value(buffer));
                buffer = "";
                continue;
            }
            break;
        default:
            break;
        }
        buffer += c;
    }
    if (!buffer.empty()) {
        tokens.push_back(TokenType::Value(buffer));
    }
    return tokens;
}

AstToken parse(std::vector<Token>&& tokens) {
    if (tokens.empty()) {
        throw std::runtime_error("No tokens to parse!");
    }

    AstToken root = AstToken(TokenType::Container{});

    for (auto it = tokens.begin(); it != tokens.end(); it++) {
        if (const auto* value = std::get_if<TokenType::Value>(&(*it))) {
            TokenType::Command token;
            while (it != tokens.end()) {
                if (const auto* value =
                        std::get_if<TokenType::Value>(&(*it))) {
                    token.arguments.push_back(value->value);
                    it++;
                } else {
                    break;
                }
            }
            it--;
            root.children.emplace_back(token);
        } else if (std::get_if<TokenType::Pipe>(&(*it))) {
            AstToken token(TokenType::Pipe{});
            token.children.push_back(root);
            root = token;
        } else if (std::get_if<TokenType::FileRedirectOut>(&(*it))) {
            AstToken token(TokenType::FileRedirectOut{});
            token.children.push_back(root);

            it++;
            if (std::get_if<TokenType::Value>(&(*it))) {
                token.children.emplace_back(std::move(*it));
            } else {
                throw std::runtime_error(
                    "Syntax error: Redirect expects to be followed by "
                    "file");
            }
            // Todo, allow other arguments and redirects to follow this
            // redirect

            root = token;

        } else {
            throw std::runtime_error(
                "Syntax error: unexpected token while parsing");
        }
    }

    return root;
}

int runAst(const Shell& shell, AstToken root) {
    if (std::get_if<TokenType::Container>(&root.type)) {
        for (auto child : root.children) {
            if (const auto rv = runAst(shell, child)) {
                return rv;
            }
            return 0;
        }
    }

    if (const auto* value = std::get_if<TokenType::Command>(&root.type)) {
        auto command = value->arguments[0];
        auto result = execute_command(shell, command, value->arguments);

        if (result) {
            return *result;
        } else {
            *(shell.err) << command << ": command not found" << std::endl;
            return 1;
        }
    }

    if (std::get_if<TokenType::Pipe>(&root.type)) {
        if (root.children.size() != 2) {
            throw std::runtime_error(
                "Syntax error: pipe('|') must have exactly two children");
        }
        Shell shell_child0 = shell;
        Shell shell_child1 = shell;

        int rv;
        if ((rv = runAst(shell_child0, root.children[0]))) {
            return rv;
        }
        if ((rv = runAst(shell_child1, root.children[1]))) {
            return rv;
        }
        return 0;
    }

    if (std::get_if<TokenType::FileRedirectOut>(&root.type)) {
        if (root.children.size() != 2) {
            throw std::runtime_error(
                "Syntax error: redirect('>') must have exactly two "
                "children");
        }

        Shell shell_child = shell;
        std::filesystem::path file_path;
        if (const auto value =
                std::get_if<TokenType::Value>(&(root.children[1].type))) {
            file_path = value->value;
        } else {
            throw std::runtime_error(
                "Syntax error: redirect('>') expects file as second "
                "child");
        }
        std::shared_ptr<std::ofstream> stream_out =
            std::make_shared<std::ofstream>(file_path);
        shell_child.out = stream_out;

        if (const auto rv = runAst(shell_child, root.children[0])) {
            return rv;
        }
        return 0;
    }

    throw std::runtime_error(
        "Syntax error: unexpected token while running");
}

int Shell::run(std::string&& source) {
    try {
        auto tokens = tokenize(std::move(source));

        if (tokens.size() == 0) {
            return 0;
        }

        auto root = parse(std::move(tokens));

        return runAst(*this, root);
    } catch (std::exception& e) {
        *(this->err) << e.what() << std::endl;
        return 1;
    }
}
