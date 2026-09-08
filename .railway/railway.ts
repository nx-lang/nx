/**
 * The nxlang.org Railway project, as code.
 *
 * Railway reads this file through `railway config plan` and `railway config apply` (from the
 * repository root, with the CLI linked to the project). It is the one place the hosting side of the
 * playground is decided — how the image is built, the health check that gates a new deployment,
 * and the restart policy the watchdog relies on — so a fresh account can be set up from it, and a
 * change to any of that is a reviewed commit. What triggers a deploy is not decided here: the
 * service has no repository source, and .github/workflows/deploy-playground.yml uploads each
 * push to main with `railway up`, as the other Railway-hosted sites do. The one-time steps that
 * live outside Railway (Cloudflare records and rules) are in docs/deployment-setup.md.
 */
import { defineRailway, empty, project, service } from "railway/iac";

export default defineRailway(() => {
  const playground = service("playground", {
    // Deployed by upload from GitHub Actions, not from a connected repository.
    source: empty(),
    // The Dockerfile copies workspace members from all over the repository, so the service's root
    // stays the repository root and the Dockerfile is named by its path from there.
    build: {
      builder: "DOCKERFILE",
      dockerfilePath: "sites/playground/Dockerfile",
    },
    deploy: {
      // Polled while a deployment starts; traffic switches only once it answers 200. Railway does
      // not poll it afterwards — replacing a stuck process is the site's own watchdog's job.
      healthcheckPath: "/playground/api/health",
      healthcheckTimeout: 60,
      // The watchdog ends a stuck process with a failure exit; this is what brings it back. ALWAYS
      // rather than ON_FAILURE with a retry budget: Railway does not say the budget resets while a
      // deployment stays live, and a service that stops restarting after its tenth hang is the
      // outcome the watchdog exists to prevent. A build that cannot serve at all never goes live,
      // because the health check above gates it, so an unbounded policy cannot loop on a broken image.
      restartPolicyType: "ALWAYS",
    },
    // The custom domain (nxlang.org) cannot be declared here — Railway's configuration rejects it —
    // so it is added once in the dashboard or with `railway domain nxlang.org --service playground`.
    // The service has no Railway-generated domain and must not be given one: it would answer
    // outside Cloudflare, where the rate limit on the API path does not apply.
    replicas: { "us-east4-eqdc4a": 1 },
  });

  return project("nxlang", {
    resources: [playground],
  });
});
