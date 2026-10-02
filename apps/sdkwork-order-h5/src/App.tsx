import { BrowserRouter } from "react-router-dom";
import { SdkworkSessionAuthBrowserRoot } from "@sdkwork/auth-pc-react";
import { SdkworkOrderH5MobileShell } from "@sdkwork/order-h5-shell";

import { AppRoutes } from "./routes/AppRoutes";
import { AuthGate } from "./AuthGate";
import { createSdkworkOrderH5Runtime } from "./bootstrap/runtime";

const runtime = createSdkworkOrderH5Runtime();

export function App() {
  return (
    <BrowserRouter>
      <SdkworkSessionAuthBrowserRoot>
        <AuthGate runtime={runtime}>
          <SdkworkOrderH5MobileShell runtime={runtime}>
            <AppRoutes runtime={runtime} />
          </SdkworkOrderH5MobileShell>
        </AuthGate>
      </SdkworkSessionAuthBrowserRoot>
    </BrowserRouter>
  );
}
