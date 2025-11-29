const { contextBridge } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    // Add any necessary APIs here
});
