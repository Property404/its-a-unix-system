SHELL = /bin/sh
COMPILER = emcc

# Where the files are
SRC_DIR = ./src/
OBJ_DIR = ./obj/
DIST_DIR = ./dist/

EXECUTABLE_NAME = core.js
EXECUTABLE_PATH = $(DIST_DIR)/$(EXECUTABLE_NAME)

# Compiler flags
CXXFLAGS = -Wall -Wextra -fexceptions 
OBJECTS = $(addprefix $(OBJ_DIR)/,main.o commands.o js_bindings.o shell.o)
LINK_FLAGS = --preload-file assets -s \
			 EXPORTED_RUNTIME_METHODS="['ccall']" -s ALLOW_MEMORY_GROWTH \
			 -s MAXIMUM_MEMORY=500MB

# Build recipe
$(EXECUTABLE_NAME): $(OBJ_DIR) $(OBJECTS)
	$(COMPILER) -o $(EXECUTABLE_PATH) $(OBJECTS) $(CXXFLAGS) $(LINK_FLAGS)
	rm -rf $(DIST_DIR)/static/
	cp -r static/* $(DIST_DIR)/
$(OBJECTS): $(OBJ_DIR)%.o : $(SRC_DIR)%.cpp
	$(COMPILER) -c -o $@ $^ $(CXXFLAGS)
$(OBJ_DIR):
	mkdir $(OBJ_DIR)
	mkdir $(DIST_DIR)
format:
	find src \( -name '*.h' -o -name '*.cpp' \) -exec clang-format -i {} \;
clean: $(OBJ_DIR)
	rm -rf $(OBJ_DIR)
	rm -rf $(DIST_DIR)
