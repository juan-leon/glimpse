import React, { useEffect, useState, useCallback, useRef } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

const GlimpseMetrics = () => {
  const [metrics, setMetrics] = useState([]);
  const [error, setError] = useState(null);
  const [isConnected, setIsConnected] = useState(false);
  const glimpseAppRef = useRef(null);

  const handleMetricsUpdate = useCallback((data) => {
    try {
      const parsedData = JSON.parse(data);
      setMetrics(parsedData.metrics);
      setError(null);
    } catch (err) {
      setError(`Failed to parse metrics: ${err.message}`);
    }
  }, []);

  useEffect(() => {
    const initializeWebSocket = async () => {
      try {
        // Initialize WASM app if not already done
        if (!window.glimpseApp) {
          throw new Error('WASM module not initialized');
        }

        glimpseAppRef.current = window.glimpseApp;

        // Connect to WebSocket
        const wsUrl = `ws://${window.location.hostname}:8080/ws`;
        await glimpseAppRef.current.connect(wsUrl, handleMetricsUpdate);
        setIsConnected(true);
        setError(null);
      } catch (err) {
        setError(`Connection failed: ${err.message}`);
        setIsConnected(false);
      }
    };

    initializeWebSocket();

    // Cleanup function
    return () => {
      if (glimpseAppRef.current) {
        glimpseAppRef.current.disconnect();
        setIsConnected(false);
      }
    };
  }, [handleMetricsUpdate]);

  const handleReconnect = useCallback(() => {
    if (glimpseAppRef.current) {
      glimpseAppRef.current.disconnect();
    }
    setIsConnected(false);
    setError(null);

    // Attempt to reconnect
    const wsUrl = `ws://${window.location.hostname}:8080/ws`;
    glimpseAppRef.current?.connect(wsUrl, handleMetricsUpdate)
      .then(() => setIsConnected(true))
      .catch(err => setError(`Reconnection failed: ${err.message}`));
  }, [handleMetricsUpdate]);

  return (
    <div className="p-4">
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-2xl font-bold">Glimpse Metrics</h2>
        <div className="flex items-center gap-4">
          <span className={`inline-block w-3 h-3 rounded-full ${
            isConnected ? 'bg-green-500' : 'bg-red-500'
          }`} />
          <span className="text-sm text-gray-600">
            {isConnected ? 'Connected' : 'Disconnected'}
          </span>
          {!isConnected && (
            <button
              onClick={handleReconnect}
              className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition"
            >
              Reconnect
            </button>
          )}
        </div>
      </div>

      {error && (
        <div className="p-4 mb-4 bg-red-50 text-red-700 rounded-md">
          {error}
        </div>
      )}

      <div className="h-96">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart
            data={metrics}
            margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
          >
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="timestamp"
              tickFormatter={(timestamp) => new Date(timestamp * 1000).toLocaleTimeString()}
            />
            <YAxis />
            <Tooltip
              labelFormatter={(timestamp) => new Date(timestamp * 1000).toLocaleString()}
              contentStyle={{ background: 'rgba(255, 255, 255, 0.9)' }}
            />
            <Line
              type="monotone"
              dataKey="value"
              stroke="#8884d8"
              activeDot={{ r: 8 }}
              isAnimationActive={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default GlimpseMetrics;
