import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@/styles/globals.css";
import { AppProvider } from "@/state/AppContext";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { App } from "@/App";

const container = document.getElementById("root");
if (!container) {
  throw new Error("The launcher root element is missing from index.html");
}

createRoot(container).render(
  <StrictMode>
    <AppProvider>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
    </AppProvider>
  </StrictMode>,
);
