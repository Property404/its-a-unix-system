const terminal = document.getElementById("terminal")
const ESCAPE_ENUM = {
    COLOR: "COLOR",
    CLEAR: "CLEAR",
    CURSOR_RELATIVE: "CURSOR_RELATIVE"
};
const DIRECTION = {
    UP:"A",
    DOWN:"B",
    RIGHT:"C",
    LEFT: "D",
};
// VERY IMPORTANT:
// 'ct' stands for 'colored terminal', NOT Connecticut
let style = "ct-normal";
let esc_sequence = null;
let cursorx = 0;

function match_escape(c) {
    const MAX_SIZE = 5;
    esc_sequence += c;

    if (esc_sequence >= MAX_SIZE) {
        esc_sequence = null;
        return null;
    }

    let result = null;
    if (result = /\[([0-9]+)m/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.COLOR,
            fg: result[1]
        }
    } else if (result = /c/.exec(esc_sequence)) {
        result = {
            type: ESCAPE_ENUM.CLEAR,
        }
    } else if (result = /\[([ABCD])/.exec(esc_sequence)) {
        console.dir(result);
        result = {
            type: ESCAPE_ENUM.CURSOR_RELATIVE,
            direction: result[1]
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
                style += " ";
            } else if (result.type = ESCAPE_ENUM.CURSOR_RELATIVE) {
                if (result.direction === DIRECTION.LEFT) {
                    cursorx--
                } else if (result.direction === DIRECTION.RIGHT) {
                    cursorx++;
                }
            }
        }
    }
    write_with_style(buffer);
}

function line_from_last(n) {
    let latest_line = terminal.lastChild;

    if (latest_line === null) {
        latest_line = document.createElement("span");
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
    let adj_span = null;
    let focus = null;
    let position = 0;

    if (str === "") {
        return;
    }

    for (child of line.children) {
        const next_position = position + child.textContent.length;
        if (next_position >= cursorx) {
            const pos = cursorx - position;
            const content = child.textContent;
            // Split divs
            // Can be optimized, probably.
            child.textContent = content.substr(0, pos);
            focus = document.createElement("span");
            focus.className = style;
            adj_span = document.createElement("span");
            adj_span.textContent = content.substr(pos+ 1);
            adj_span.className = child.className;
            line.insertBefore(focus, child.nextSibling);
            line.insertBefore(adj_span, focus.nextSibling);
            break;
        }
        position = next_position;
    }

    if (focus === null) {
        console.log("[Tacking on]")
        console.log("cursorx", cursorx)
        console.log("position", position)
        padding = document.createElement("span");
        padding.textContent = "~".repeat(cursorx-position);
        line.appendChild(padding);
        
        focus = document.createElement("span");
        focus.className = style;
        line.appendChild(focus);
    }

    for (i in str) {
        const c = str[i];
        if (c == '\n') {
            let new_line = document.createElement("span");
            terminal.insertBefore(new_line, line.nextSibling);
            terminal.insertBefore(document.createTextNode("\n"), new_line);
            cursorx = 0;
            return write_to_line(new_line, str.substr(i+1));
        }
        focus.textContent += c;
        cursorx += 1;
        while(adj_span?.textContent === "") {
            let temp = adj_span;
            adj_span = adj_span.nextSibling;
            line.removeChild(temp);
        }
        if (adj_span === null) {
            continue;
        }
        adj_span.textContent = adj_span.textContent.substr(1);
    }
    console.log(cursorx);
}

function write_with_style(str) {
    const latest_line = line_from_last(0);
    write_to_line(latest_line, str);
    terminal.scrollTop = terminal.scrollHeight;
}

function js_term_clear() {
    terminal.innerHTML = "";
    cursorx = 0;
}

function js_term_backspace() {
    const latest_div = terminal.lastChild.lastChild;
    const text = latest_div.textContent
    if (text !== "") {
        latest_div.textContent = text.substr(0, text.length - 1);
    }
    if (latest_div.textContent === "") {
        latest_div.remove();
    }
    cursorx -= 1;
}
