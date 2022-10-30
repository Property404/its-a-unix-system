const context = {
    terminal: document.getElementById("terminal"),
    buffer: ""
};

function addText(content) {
    context.terminal.textContent += content;
}

function newPrompt() {
    context.buffer = "";
}

var Module = {
    onRuntimeInitialized: () => {
        const e = document.getElementById('loadingDiv');
        e.style.display = 'none';
    }, 
    print: (content,v) => {
        console.log("PRINT: '"+content+"'");
    }
};

function jsPrint(content) {
    addText(content);
}

document.addEventListener("keydown", (e) => {
    if ("/?'".includes(e.key)) {
        e.preventDefault();
    }
    const printable = 
        (e.keyCode > 47 && e.keyCode < 58)   || // number keys
        e.keyCode === 173 || e.keyCode === 61   || //-_+=
        e.keyCode === 32 ||  e.keyCode === 59 || // space, colon
        (e.keyCode > 64 && e.keyCode < 91)   || // letter keys
        (e.keyCode > 95 && e.keyCode < 112)  || // numpad keys
        (e.keyCode > 185 && e.keyCode < 193) || // ;=,-./` (in order)
        (e.keyCode > 218 && e.keyCode < 223);   // [\]' (in order)

    if (printable) {
        context.buffer += e.key;
        addText(e.key);
    } else {
        console.log(e);
    }

    // Delete
    if (e.keyCode === 0x08)  {
        function popoff(val) {
            return val.substr(0, val.length - 1);
        }
        if (context.buffer.length > 0) {
            context.buffer = popoff(context.buffer);
            context.terminal.textContent = popoff(context.terminal.textContent);
        }
    }

    if (e.keyCode === 0x0d)  {
        addText("\n");
        const result = Module.ccall(
                  "process_line", // name of C function
                  "number", // return type
                  ["string"], // argument types
                  [context.buffer] // arguments
                );
        newPrompt();
    }
})

newPrompt();
