# Kubewarden policy context-aware-test-policy

## Description

This is a test policy used in the policy-evaluator integration tests.
Every time a deployment with the label `app.kubernetes.io/component: "api"` is created or updated it checks the following:

- The Deployment must have a `customer-id` label set.
- The value of the `customer-id` label of the deployment must match the value of the `customer-id` namespace where the deployment has been created.
- A deployment with the label `app.kubernetes.io/component: database` must exist in the deployment namespace.
- A deployment with the label `app.kubernetes.io/component: frontend` must exist in the deployment namespace.
- A service named `api-auth-service` with the label `app.kubernetes.io/part-of: api` must exist in the deployment namespace.
- Check if it can get the OCI manifest for the policy `ghcr.io/kubewarden/tests/context-aware-test-policy:latest`
- Check if it can get the OCI image index manifest for the container image `ghcr.io/kubewarden/policy-server:latest`

## Settings

This policy has no configurable settings.
