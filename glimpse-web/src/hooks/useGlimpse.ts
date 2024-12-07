// glimpse-web/src/hooks/useGlimpse.ts
import { useState, useEffect, useCallback } from 'react';
import type { Metric } from '../types/glimpse';

export function useGlimpse(wsUrl: string) {
    const [metrics, setMetrics] = useState<Metric[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [isConnected, setIsConnected] = useState(false);

    const handleMetricsUpdate = useCallback((data: string) => {
        try {
            const parsedData = JSON.parse(data);
            setMetrics(parsedData.metrics);
            setError(null);
        } catch (err) {
            setError(`Failed to parse metrics: ${err instanceof Error ? err.message : 'Unknown error'}`);
        }
    }, []);

    useEffect(() => {
        const initializeWebSocket = async () => {
            if (!window.glimpseApp) {
                setError('WASM module not initialized');
                return;
            }

            try {
                await window.glimpseApp.connect(wsUrl, handleMetricsUpdate);
                setIsConnected(true);
                setError(null);
            } catch (err) {
                setError(`Connection failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
                setIsConnected(false);
            }
        };

        initializeWebSocket();

        return () => {
            window.glimpseApp?.disconnect();
            setIsConnected(false);
        };
    }, [wsUrl, handleMetricsUpdate]);

    const reconnect = useCallback(async () => {
        window.glimpseApp?.disconnect();
        setIsConnected(false);
        setError(null);

        try {
            await window.glimpseApp?.connect(wsUrl, handleMetricsUpdate);
            setIsConnected(true);
        } catch (err) {
            setError(`Reconnection failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
        }
    }, [wsUrl, handleMetricsUpdate]);

    return { metrics, error, isConnected, reconnect };
}
