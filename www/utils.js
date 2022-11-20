const terminal = document.getElementById("terminal");

function js_term_write(str) {
    terminal.textContent += str;
}

function js_term_clear() {
    terminal.textContent = "";
}

function js_term_backspace() {
    terminal.textContent =
        terminal.textContent.substr(0, terminal.textContent.length - 1);
}
