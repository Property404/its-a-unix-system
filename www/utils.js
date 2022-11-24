const terminal = document.getElementById("terminal");

// VERY IMPORTANT:
// 'ct' stands for 'colored terminal', NOT Connecticut
let style = "ct-normal";
let esc_sequence = "";

function js_term_write(str) {
    let buffer = "";
    for (c of str) {
        if (esc_sequence.length === 0) {
            if (c === "\u001b") {
                write_with_style(buffer);
                buffer = "";
                esc_sequence += c;
            } else {
                buffer += c;
            }
        } else {
            if (c === "m")  {
                let esc = esc_sequence.replace("\u001b[","");
                if (esc === "30") {
                    style += "ct-black";
                } else if (esc === "31") {
                    style += "ct-red";
                } else if (esc === "32") {
                    style += "ct-green";
                } else if (esc === "33") {
                    style += "ct-yellow";
                } else if (esc === "34") {
                    style += "ct-blue";
                } else if (esc === "35") {
                    style += "ct-magenta";
                } else if (esc === "36") {
                    style += "ct-cyan";
                } else if (esc === "0") {
                    style = "ct-normal";
                }
                style += " ";
                esc_sequence = "";
            } else {
                esc_sequence += c;
            }
        }
    }
    write_with_style(buffer);
}
function write_with_style(str) {
    const latest_div = terminal.lastChild;
    let target_div = null;
    if (latest_div && latest_div.className === style) {
        target_div = latest_div;
    } else {
        target_div = document.createElement("span");
        target_div.className = style;
        terminal.appendChild(target_div);
    }
    target_div.textContent += str;
    terminal.scrollTop = terminal.scrollHeight;
}

function js_term_clear() {
    terminal.innerHTML = "";
}

function js_term_backspace() {
    const latest_div = terminal.lastChild;
    const text = latest_div.textContent
    if (text === "") {
        latest_div.remove();
    } else {
        latest_div.textContent = text.substr(0, text.length - 1);
    }
}
