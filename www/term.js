const terminal = document.getElementById("terminal")
const hidey_hole = document.getElementById("hidey-hole");
const cursor = document.getElementById("cursor");

const ESCAPE_ENUM = {
    COLOR: "COLOR",
    CLEAR: "CLEAR",
    CURSOR_RELATIVE: "CURSOR_RELATIVE",
    CLEAR_LINE: "CLEAR_LINE",
    CLEAR_TO_END: "CLEAR_TO_END",
};
const DIRECTION = {
    UP:"A",
    DOWN:"B",
    RIGHT:"C",
    LEFT: "D",
    LEFT_ABS: "G",
};
// VERY IMPORTANT:
// 'ct' stands for 'colored terminal', NOT Connecticut
let style = "ct-normal";
let esc_sequence = null;
let cursorx = 0;

function get_pos_in_line(line, x) {
    let adj_span = null;
    let hit = false;
    let position = 0;

    for (child of line.children) {
        if (child.id === cursor.id) {
            continue;
        }
        const next_position = position + child.textContent.length;
        if (next_position >= x) {
            const pos = x - position;
            const content = child.textContent;
            hit = true;
            // Split spans
            // Can be optimized, probably.
            child.textContent = content.substr(0, pos);
            if (pos != content.length) {
                adj_span = document.createElement("span");
                adj_span.textContent = content.substr(pos);
                adj_span.className = child.className;
                line.insertBefore(adj_span, child.nextSibling);
            }
            return child;
        }
        position = next_position;
    }

    padding = document.createElement("span");
    padding.textContent = "*".repeat(x-position);
    line.appendChild(padding);
    return padding;
}

function move_cursor_x(x) {
    if (x < 0) {
        return;
    }
    cursorx = x;
    hidey_hole.appendChild(cursor);

    const line = line_from_last(0);
    const span = get_pos_in_line(line, cursorx);

    line.insertBefore(cursor, span.nextSibling);
}

function match_escape(c) {
    const MAX_SIZE = 5;
    esc_sequence += c;

    if (esc_sequence >= MAX_SIZE) {
        esc_sequence = null;
        return null;
    }

    let result = null;
    // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
    if (result = /\[([0-9]+)m/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.COLOR,
            fg: result[1]
        }
    } else if (result = /c/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.CLEAR,
        }
    } else if (result = /\[([ABCDG])/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.CURSOR_RELATIVE,
            direction: result[1]
        }
    } else if (result = /\[2K/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.CLEAR_LINE
        }
    } else if (result = /\[0K/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.CLEAR_TO_END
        }
    }

    if (result !== null) {
        esc_sequence = null;
    }

    return result;
}

function js_term_write(str) {
    let buffer = "";
    for (c of str) {
        if (esc_sequence === null) {
            if (c === "\u001b") {
                write_with_style(buffer);
                buffer = "";
                esc_sequence = "";
            } else {
                buffer += c;
            }
        } else if (result = match_escape(c)) {
            if (result.type === ESCAPE_ENUM.COLOR)  {
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
                } else {
                    console.error("UNIMPLEMENTED ANSI CODE DIRECTION", result.direction);
                }
            } else if (result.type == ESCAPE_ENUM.CLEAR) {
                js_term_clear();
            } else if (result.type == ESCAPE_ENUM.CLEAR_LINE) {
                if (line_to_clear = terminal.lastChild) {
                    line_to_clear.replaceChildren()
                }
            } else if (result.type == ESCAPE_ENUM.CLEAR_TO_END) {
                move_cursor_x(cursorx);
                const line = cursor.parentElement;
                let element = cursor.nextSibling;
                while (element !== null) {
                    temp = element;
                    element = element.nextSibling;
                    line.removeChild(temp);
                }
            } else {
                console.error("UNIMPLEMENTED ANSI CODE", result);
            }
        }
    }
    write_with_style(buffer);
    move_cursor_x(cursorx);
}

function line_from_last(n) {
    let latest_line = terminal.lastChild;

    if (latest_line === null) {
        latest_line = document.createElement("div");
        terminal.appendChild(latest_line);
    }

    if (n < 0) {
        let sibling = latest_line;
        while (n++ < 0) {
            sibling = sibling.previousSibiling;
            if (sibling  == null) {
                break;
            }
        }
        if (sibling === null) {
            sibling = terminal.firstChild;
        }
        return sibling;
    }

    if (n > 0) {
        while (n-- > 0) {
            latest_line = document.createElement("span");
            terminal.appendChild(latest_line);
        }
        return latest_line;
    }

    return latest_line;
}


function write_to_line(line, str) {
    let focus = null;
    let position = 0;

    if (str === "") {
        return;
    }

    hidey_hole.appendChild(cursor);
    const adj_span = get_pos_in_line(line, cursorx).nextSibling;
    focus = document.createElement("span");
    focus.className = style;
    line.insertBefore(focus, adj_span);

    for (let i = 0; i < str.length; i++) {
        const c = str[i];
        if (c == '\n') {
            let new_line = document.createElement("div");
            terminal.insertBefore(new_line, line.nextSibling);
            cursorx = 0;
            write_to_line(new_line, str.substr(i+1));
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
        while(adj_span?.textContent === "") {
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
    const latest_line = line_from_last(0);
    write_to_line(latest_line, str);
    terminal.scrollTop = terminal.scrollHeight;
}

function js_term_clear() {
    terminal.innerHTML = "";
    move_cursor_x(0);
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
    move_cursor_x(cursorx - 1);
}
