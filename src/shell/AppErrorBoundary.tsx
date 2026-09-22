/**
 * App error boundary: a crash shows a readable diagnostic screen
 * instead of a blank window (production-grade resilience), with the
 * component stack so failures are debuggable from a screenshot.
 */
import { Component, type ErrorInfo, type ReactNode } from "react";

interface State {
  error: Error | null;
}

export class AppErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("[DiskBytes] UI crashed:", error, info.componentStack);
  }

  render(): ReactNode {
    if (this.state.error) {
      return (
        <div
          style={{
            height: "100vh",
            display: "grid",
            placeItems: "center",
            background: "#1e1e20",
            color: "#f5f5f7",
            fontFamily: "Consolas, monospace",
            padding: 40,
          }}
        >
          <div style={{ maxWidth: 900 }}>
            <h1 style={{ color: "#ff6b4a", fontSize: 20 }}>DiskBytes hit an error</h1>
            <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, lineHeight: 1.6 }}>
              {String(this.state.error?.stack ?? this.state.error)}
            </pre>
            <pre style={{ whiteSpace: "pre-wrap", fontSize: 11, opacity: 0.6 }}>
              {String((this.props as { info?: ErrorInfo }).info?.componentStack ?? "")}
            </pre>
            <button
              type="button"
              onClick={() => window.location.reload()}
              style={{ marginTop: 18, padding: "10px 22px", background: "#ff6b4a", color: "#fff", border: 0, borderRadius: 8, fontWeight: 700, cursor: "pointer" }}
            >
              Reload
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
