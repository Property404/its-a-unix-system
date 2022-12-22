"use strict";

const terminal = document.getElementById("terminal")
const hidey_hole = document.getElementById("hidey-hole");
const cursor = document.getElementById("cursor");

const ESCAPE_ENUM = {
    COLOR: "COLOR",
    CLEAR: "CLEAR",
    CURSOR_RELATIVE: "CURSOR_RELATIVE",
    CLEAR_LINE: "CLEAR_LINE",
    CLEAR_TO_END: "CLEAR_TO_END",
    ABS_POS: "ABS_POS",
    POP_TOP: "POP_TOP",
    POP_BOTTOM: "POP_BOTTOM",
    PUSH_TOP: "PUSH_TOP",
};
const DIRECTION = {
    UP: "A",
    DOWN: "B",
    RIGHT: "C",
    LEFT: "D",
    LEFT_ABS: "G",
};
// VERY IMPORTANT:
// 'ct' stands for 'colored terminal', NOT Connecticut
let style = "ct-normal";
let esc_sequence = null;
let cursorx = 0;
let cursory = null;

function get_pos_in_line(line, x) {
    let adj_span = null;
    let position = 0;

    for (const child of line.children) {
        if (child.id === cursor.id) {
            continue;
        }
        const next_position = position + child.textContent.length;
        if (next_position >= x) {
            const pos = x - position;
            const content = child.textContent;
            // Split spans
            // Can be optimized, probably.
            child.textContent = content.substr(0, pos);
            if (pos != content.length) {
                adj_span = document.createElement("span");
                adj_span.textContent = content.substr(pos);
                adj_span.className = child.className;
                line.insertBefore(adj_span, child.nextSibling);
            }
            return child.nextSibling;
        }
        position = next_position;
    }

    if (x != position) {
        const padding = document.createElement("span");
        padding.textContent = "*".repeat(x - position);
        line.appendChild(padding);
        return padding.nextSibling;
    }
    return null;
}

function move_cursor(x, y) {
    if (x < 0) {
        return;
    }
    if (y < 0) {
        return;
    }
    cursorx = x;
    cursory = y;
    hidey_hole.appendChild(cursor);

    const line = current_line();
    const span = get_pos_in_line(line, cursorx);

    line.insertBefore(cursor, span);
}

function match_escape(c) {
    const MAX_SIZE = 7;
    esc_sequence += c;

    if (esc_sequence >= MAX_SIZE) {
        esc_sequence = null;
        return null;
    }

    let result = null;
    // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
    if ((result = /\[([0-9]+)m/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.COLOR,
            fg: result[1]
        }
    } else if ((result = /c/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.CLEAR,
        }
    } else if ((result = /\[([ABCDG])/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.CURSOR_RELATIVE,
            direction: result[1]
        }
    } else if ((result = /\[2K/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.CLEAR_LINE
        }
    } else if ((result = /\[0K/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.CLEAR_TO_END
        }
    } else if ((result = /\[popt/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.POP_TOP
        }
    } else if ((result = /\[popb/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.POP_BOTTOM
        }
    } else if ((result = /\[pusht/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.PUSH_TOP
        }
    } else if ((result = /\[([0-9]+);([0-9]+)H/.exec(esc_sequence))) {
        result = {
            type: ESCAPE_ENUM.ABS_POS,
            row: +result[1],
            column: +result[2],
        }
    }

    if (result !== null) {
        esc_sequence = null;
    }

    return result;
}

function js_term_write(str) {
    let buffer = "";
    let result;
    for (const c of str) {
        if (esc_sequence === null) {
            if (c === "\u001b") {
                write_with_style(buffer);
                buffer = "";
                esc_sequence = "";
            } else {
                buffer += c;
            }
        } else if ((result = match_escape(c))) {
            if (result.type === ESCAPE_ENUM.COLOR) {
                let fg = result.fg;
                style += " ";
                if (fg === "30") {
                    style += "ct-black";
                } else if (fg === "31") {
                    style += "ct-red";
                } else if (fg === "32") {
                    style += "ct-green";
                } else if (fg === "33") {
                    style += "ct-yellow";
                } else if (fg === "34") {
                    style += "ct-blue";
                } else if (fg === "35") {
                    style += "ct-magenta";
                } else if (fg === "36") {
                    style += "ct-cyan";
                } else if (fg === "0") {
                    style = "ct-normal";
                }
            } else if (result.type === ESCAPE_ENUM.CURSOR_RELATIVE) {
                if (result.direction === DIRECTION.LEFT) {
                    if (cursorx > 0) {
                        cursorx -= 1;
                    }
                } else if (result.direction === DIRECTION.RIGHT) {
                    cursorx += 1;
                } else if (result.direction === DIRECTION.LEFT_ABS) {
                    cursorx = 0;
                } else if (result.direction === DIRECTION.UP) {
                    if (cursory > 0) {
                        cursory -= 1;
                    }
                } else if (result.direction === DIRECTION.DOWN) {
                    cursory += 1;
                } else {
                    console.error("UNIMPLEMENTED ANSI CODE DIRECTION", result.direction);
                }
            } else if (result.type == ESCAPE_ENUM.CLEAR) {
                js_term_clear();
            } else if (result.type == ESCAPE_ENUM.CLEAR_LINE) {
                current_line().replaceChildren();
            } else if (result.type == ESCAPE_ENUM.CLEAR_TO_END) {
                move_cursor(cursorx, cursory);
                const line = cursor.parentElement;
                let element = cursor.nextSibling;
                while (element !== null) {
                    const temp = element;
                    element = element.nextSibling;
                    line.removeChild(temp);
                }
            } else if (result.type == ESCAPE_ENUM.ABS_POS) {
                cursory = result.row;
                cursorx = result.column;
            } else if (result.type == ESCAPE_ENUM.POP_TOP) {
                hidey_hole.appendChild(cursor)
                terminal.removeChild(terminal.firstChild);
                move_cursor(cursorx, cursory);
            } else if (result.type == ESCAPE_ENUM.POP_BOTTOM) {
                hidey_hole.appendChild(cursor)
                terminal.removeChild(terminal.lastChild);
                move_cursor(cursorx, cursory);
            } else if (result.type == ESCAPE_ENUM.PUSH_TOP) {
                terminal.insertBefore(document.createElement("div"), terminal.firstChild);
            } else {
                console.error("UNIMPLEMENTED ANSI CODE", result);
            }
        }
    }
    write_with_style(buffer);
    move_cursor(cursorx, cursory);
}

function current_line() {
    let line;
    if (cursory === null) {
        line = terminal.lastChild;

        if (line === null) {
            line = document.createElement("div");
            terminal.appendChild(line);
        }
    } else {
        line = line_from_top(cursory);
    }

    if (line == null) {
        throw new Error("NULL LINE");
    }

    return line;
}

function line_from_top(n) {
    let line = terminal.firstChild;

    if (line === null) {
        line = document.createElement("div");
        terminal.appendChild(line);
    }

    while (n > 0) {
        line = line.nextSibling;
        if (line === null) {
            line = document.createElement("div");
            terminal.appendChild(line);
        }
        n--;
    }

    return line;
}

function write_to_line(line, str) {
    let focus = null;

    if (str === "") {
        return;
    }

    hidey_hole.appendChild(cursor);
    let adj_span = get_pos_in_line(line, cursorx);
    focus = document.createElement("span");
    focus.className = style;
    line.insertBefore(focus, adj_span);

    for (let i = 0; i < str.length; i++) {
        const c = str[i];
        if (c == '\n') {
            let new_line;
            if (cursory === null) {
                new_line = document.createElement("div");
                terminal.insertBefore(new_line, line.nextSibling);
            } else {
                cursory++;
                new_line = current_line();
            }
            cursorx = 0;
            write_to_line(new_line, str.substr(i + 1));
            return;
        }
        if (c == '\b') {
            if (cursorx > 0) {
                cursorx -= 1;
            }
            continue;
        }
        focus.textContent += c;
        cursorx += 1
        while (adj_span?.textContent === "") {
            let temp = adj_span;
            adj_span = adj_span.nextSibling;
            if (adj_span?.id === cursor.id) {
                adj_span = adj_span.nextSibling;
            }
            line.removeChild(temp);
        }
        if (adj_span === null) {
            continue;
        }
        adj_span.textContent = adj_span.textContent.substr(1);
    }
}

function write_with_style(str) {
    if (str === "") {
        return;
    }
    const latest_line = current_line();
    write_to_line(latest_line, str);
    terminal.scrollTop = terminal.scrollHeight;
}

function js_term_clear() {
    terminal.innerHTML = "";
    move_cursor(0, null);
}

function js_term_backspace() {
    let latest_div = terminal.lastChild.lastChild;
    if (latest_div === cursor) {
        latest_div = latest_div.previousSibling;
    }
    const text = latest_div.textContent
    if (text !== "") {
        latest_div.textContent = text.substr(0, text.length - 1);
    }
    if (latest_div.textContent === "") {
        latest_div.remove();
    }
    move_cursor(cursorx - 1, cursory);
}

// Not even remotely accurate, but at least it's usually less than the
// actual screen height.
function js_term_get_screen_height() {
    function rem_to_pixels(rem) {
        return rem * parseFloat(getComputedStyle(document.documentElement).fontSize);
    }

    const rem = rem_to_pixels(1);
    const lines = Math.round((terminal.offsetHeight / rem) * 0.5);
    return lines;
}
