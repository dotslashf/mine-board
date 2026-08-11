import { createRootRoute, createRoute, createRouter, Outlet } from "@tanstack/react-router";

import { Shell } from "./components/Shell";
import { SoundboardPage } from "./routes/soundboard";
import { SettingsPage } from "./routes/settings";

const rootRoute = createRootRoute({
  component: () => (
    <Shell>
      <Outlet />
    </Shell>
  ),
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: SoundboardPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([indexRoute, settingsRoute]);

export const router = createRouter({ routeTree });
