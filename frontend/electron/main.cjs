const { app, BrowserWindow } = require('electron');
const path = require('path');
const { spawn } = require('child_process');

let mainWindow;
let rustProcess;

function createWindow() {
    mainWindow = new BrowserWindow({
        width: 800,
        height: 600,
        webPreferences: {
            preload: path.join(__dirname, 'preload.cjs'),
            nodeIntegration: false,
            contextIsolation: true,
        },
    });

    const isDev = !app.isPackaged;
    const startUrl = isDev
        ? 'http://localhost:3000'
        : `file://${path.join(__dirname, '../dist/index.html')}`;

    mainWindow.loadURL(startUrl);

    if (isDev) {
        mainWindow.webContents.openDevTools();
    }
}

function startRustService() {
    const isDev = !app.isPackaged;
    // Adjust path based on dev (relative to project root) or prod (bundled)
    const binaryPath = isDev
        ? path.join(__dirname, '../../backend/target/debug/rust-service.exe')
        : path.join(process.resourcesPath, 'rust-service.exe');

    console.log(`Starting Rust service from: ${binaryPath}`);

    rustProcess = spawn(binaryPath, [], {
        cwd: isDev ? path.join(__dirname, '../../backend') : path.dirname(binaryPath),
    });

    rustProcess.stdout.on('data', (data) => {
        console.log(`Rust: ${data}`);
    });

    rustProcess.stderr.on('data', (data) => {
        console.error(`Rust Error: ${data}`);
    });

    rustProcess.on('close', (code) => {
        console.log(`Rust process exited with code ${code}`);
    });
}

app.whenReady().then(() => {
    startRustService();
    createWindow();

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) {
            createWindow();
        }
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});

app.on('will-quit', () => {
    if (rustProcess) {
        rustProcess.kill();
    }
});
