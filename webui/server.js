const express = require('express');
const cors = require('cors');
const path = require('path');

const app = express();
const port = 8080;

app.use(cors());  // 启用所有来源的 CORS

// 提供静态文件服务
app.use(express.static(path.join(__dirname, '.')));

app.listen(port, () => {
    console.log(`Server running at http://127.0.0.1:${port}/`);
});