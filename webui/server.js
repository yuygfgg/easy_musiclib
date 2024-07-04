const express = require('express');
const cors = require('cors');
const path = require('path');

const app = express();
const port = 8080;

app.use(cors());

app.use(express.static(path.join(__dirname, '.')));

app.listen(port, () => {
    console.log(`Server running at http://127.0.0.1:${port}/`);
});