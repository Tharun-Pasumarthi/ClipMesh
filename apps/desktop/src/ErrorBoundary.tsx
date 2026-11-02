import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ClipMesh Uncaught Rendering Error:", error, errorInfo);
  }

  private handleReset = () => {
    try {
      localStorage.removeItem("clipmesh-devices");
      localStorage.removeItem("clipmesh-clips");
    } catch (e) {
      console.error(e);
    }
    window.location.reload();
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: "100vh",
          background: "#0b0e14",
          color: "#e2e8f0",
          fontFamily: "system-ui, -apple-system, sans-serif",
          padding: "24px",
          textAlign: "center"
        }}>
          <div style={{
            background: "#161b22",
            border: "1px solid #30363d",
            borderRadius: "12px",
            padding: "32px",
            maxWidth: "480px"
          }}>
            <h2 style={{ fontSize: "20px", fontWeight: 600, marginBottom: "8px", color: "#f87171" }}>
              Application Render Issue Caught
            </h2>
            <p style={{ color: "#94a3b8", fontSize: "14px", lineHeight: "1.5", marginBottom: "20px" }}>
              {this.state.error?.message || "An unexpected error occurred while rendering the workspace."}
            </p>
            <button
              onClick={this.handleReset}
              style={{
                background: "#6366f1",
                color: "#ffffff",
                border: "none",
                borderRadius: "8px",
                padding: "10px 20px",
                fontSize: "14px",
                fontWeight: 500,
                cursor: "pointer",
              }}
            >
              Reset Cached Data & Reload
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
