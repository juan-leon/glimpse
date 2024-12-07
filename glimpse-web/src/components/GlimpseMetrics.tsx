import React from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useGlimpse } from '../hooks/useGlimpse';

const GlimpseMetrics: React.FC = () => {
    const wsUrl = `ws://${window.location.hostname}:8080/ws`;
    const { metrics, error, isConnected, reconnect } = useGlimpse(wsUrl);

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
                            onClick={reconnect}
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
