export interface Metric {
    url: string;
    value: number;
    timestamp: number;
}

export interface MetricsData {
    metrics: Metric[];
}

export interface GlimpseApp {
    connect(url: string, callback: (data: string) => void): Promise<void>;
    disconnect(): void;
    update_metrics(json: string): Promise<void>;
    get_metrics_json(): string;
}

declare global {
    interface Window {
        glimpseApp: GlimpseApp;
    }
}
