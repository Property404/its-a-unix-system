const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
    entry: "./bootstrap.js",
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "bootstrap.js",
    },
    mode: "production",
    experiments: {
        asyncWebAssembly: true
    },
    plugins: [ 
        new CopyWebpackPlugin(
            {
                "patterns": ['index.html', 'term.js', 'style.css']
            }
        )
    ],
    performance: {
        maxAssetSize: 5000000,
    }
};
