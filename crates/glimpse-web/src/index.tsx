import React from 'react';
import { createRoot } from 'react-dom/client';
import GlimpseMetrics from './components/GlimpseMetrics';

// Initialize WASM
import init from '../pkg/glimpse_web';

async function start() {
    // Initialize the WASM module
    await init();

    // Create React root and render
    const container = document.getElementById('root');
    if (container) {
        const root = createRoot(container);
        root.render(
            <React.StrictMode>
                <GlimpseMetrics />
            </React.StrictMode>
        );
    }
}

start().catch(console.error);
