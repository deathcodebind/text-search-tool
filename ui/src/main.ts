import App from "./App.svelte";
import "../styles.css";
import "@shoelace-style/shoelace/dist/themes/light.css";
import "@shoelace-style/shoelace/dist/shoelace.js";

function showFatalError(title: string, detail: unknown) {
  console.error(title, detail);
  const container = document.getElementById("app");
  if (container) {
    container.innerHTML = `\n      <div style="padding: 24px; font-family: ui-sans-serif, system-ui, sans-serif; background: #1e1e2f; color: #f8f8ff; min-height: 100vh;">
        <h1 style="margin-top: 0;">应用初始化失败</h1>
        <p>${title}</p>
        <pre style="white-space: pre-wrap; word-break: break-word; background: rgba(255,255,255,0.08); padding: 12px; border-radius: 10px;">${String(detail)}</pre>
      </div>
    `;
  }
}

function logGlobalError(event: ErrorEvent) {
  const error = event.error || event.message;
  console.error("[GLOBAL ERROR]", error, event);
  showFatalError("GLOBAL ERROR", error);
}

function logUnhandledRejection(event: PromiseRejectionEvent) {
  console.error("[GLOBAL UNHANDLED REJECTION]", event.reason, event);
  showFatalError("GLOBAL UNHANDLED REJECTION", event.reason);
}

window.addEventListener("error", logGlobalError, true);
window.addEventListener("unhandledrejection", logUnhandledRejection, true);
window.onerror = (message, source, lineno, colno, error) => {
  const payload = error || `${message} at ${source}:${lineno}:${colno}`;
  logGlobalError(new ErrorEvent("error", { error: payload, message: String(message) }));
  return false;
};
window.onunhandledrejection = (event) => {
  logUnhandledRejection(event as PromiseRejectionEvent);
};

let app: App;
try {
  app = new App({
    target: document.getElementById("app") as HTMLElement,
  });
} catch (error) {
  showFatalError("APP INIT FAILED", error);
  throw error;
}

export default app;
