import { Component } from 'react';
import './ErrorBoundary.css';

// Top-level safety net: a render throw anywhere below would otherwise leave a
// blank window with no system chrome to close it (decorations are off). Catch
// it, show the error plus a way to copy it and reload the webview.
export default class ErrorBoundary extends Component {
  constructor(props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  componentDidCatch(error, info) {
    // eslint-disable-next-line no-console
    console.error('Unhandled UI error:', error, info);
  }

  render() {
    if (!this.state.error) return this.props.children;

    const text = String(this.state.error?.stack || this.state.error?.message || this.state.error);
    return (
      <div className="error-boundary">
        <div className="error-boundary__card">
          <div className="error-boundary__title">Something went wrong</div>
          <p className="error-boundary__desc">
            The launcher hit an unexpected error and cannot continue. You can copy the
            details for a bug report, then reload.
          </p>
          <pre className="error-boundary__detail">{text}</pre>
          <div className="error-boundary__actions">
            <button
              className="error-boundary__btn"
              onClick={() => {
                navigator.clipboard?.writeText(text).catch(() => {});
              }}
            >
              Copy details
            </button>
            <button
              className="error-boundary__btn error-boundary__btn--primary"
              onClick={() => window.location.reload()}
            >
              Reload
            </button>
          </div>
        </div>
      </div>
    );
  }
}
