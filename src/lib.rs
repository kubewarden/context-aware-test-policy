use k8s_openapi::api::apps::v1::Deployment;
use kubewarden::host_capabilities::kubernetes::{list_all_resources, list_resources_by_namespace};
use lazy_static::lazy_static;

use guest::prelude::*;
use kubewarden_policy_sdk::wapc_guest as guest;

use k8s_openapi::api::core::v1::{Namespace, Service};
use k8s_openapi::{List, Metadata};

extern crate kubewarden_policy_sdk as kubewarden;
use kubewarden::{
    host_capabilities::kubernetes::{
        get_resource, GetResourceRequest, ListAllResourcesRequest, ListResourcesByNamespaceRequest,
    },
    host_capabilities::oci::manifest,
    logging, protocol_version_guest,
    request::ValidationRequest,
    validate_settings,
};

mod settings;
use settings::Settings;

use slog::{o, Logger};

lazy_static! {
    static ref LOG_DRAIN: Logger = Logger::root(
        logging::KubewardenDrain::new(),
        o!("policy" => "sample-policy")
    );
}

#[no_mangle]
pub extern "C" fn wapc_init() {
    register_function("validate", validate);
    register_function("validate_settings", validate_settings::<Settings>);
    register_function("protocol_version", protocol_version_guest);
}

fn validate(payload: &[u8]) -> CallResult {
    let validation_request: ValidationRequest<Settings> = ValidationRequest::new(payload)?;
    let deployment = serde_json::from_value::<Deployment>(validation_request.request.object)?;

    let labels = if let Some(labels) = deployment.metadata.labels.clone() {
        labels
    } else {
        return kubewarden::accept_request();
    };

    // Check if the app.kubernetes.io/component label is set to "api"
    // If not, accept the request
    if labels.get("app.kubernetes.io/component") != Some(&"api".to_owned()) {
        return kubewarden::accept_request();
    }

    // Get the customer_id label value
    // If not set, reject the request
    let customer_id = if let Some(customer_id) = labels.get("customer-id") {
        customer_id
    } else {
        return kubewarden::reject_request(
            Some("Label customer-id is required for API deployments".to_owned()),
            Some(400),
            None,
            None,
        );
    };

    // Check if the namespace of the deployment has the label customer_id
    let kube_request = ListAllResourcesRequest {
        api_version: "v1".to_owned(),
        kind: "Namespace".to_owned(),
        label_selector: Some(format!("customer-id={}", customer_id)),
        field_selector: None,
    };

    let namespaces: List<Namespace> = list_all_resources(&kube_request)?;
    if namespaces.items.is_empty() {
        return kubewarden::reject_request(
            Some(format!(
                "Label customer-id ({}) must match namespace label",
                customer_id
            )),
            Some(404),
            None,
            None,
        );
    }

    if namespaces.items.len() > 1 {
        return kubewarden::reject_request(
            Some(format!(
                "Multiple namespaces found with label 'customer-id={}'",
                customer_id
            )),
            Some(400),
            None,
            None,
        );
    }

    let namespace = namespaces.items[0].metadata().name.as_ref().unwrap();
    if deployment.metadata().namespace != Some(namespace.clone()) {
        return kubewarden::reject_request(
            Some("Deployment must be created in the matching customer namespace".to_owned()),
            Some(400),
            None,
            None,
        );
    }

    let kube_request = ListResourcesByNamespaceRequest {
        api_version: "apps/v1".to_owned(),
        kind: "Deployment".to_owned(),
        namespace: namespace.clone(),
        label_selector: None,
        field_selector: None,
    };
    let deployments: List<Deployment> = list_resources_by_namespace(&kube_request)?;

    // Check if the namespace has a database and a frontend component deployed
    if !deployments.items.iter().any(|deployment| {
        if let Some(labels) = deployment.metadata().labels.clone() {
            labels.get("app.kubernetes.io/component") == Some(&"database".to_owned())
        } else {
            false
        }
    }) {
        return kubewarden::reject_request(
            Some("No database component found".to_owned()),
            Some(404),
            None,
            None,
        );
    }

    if !deployments.items.iter().any(|deployment| {
        if let Some(labels) = deployment.metadata().labels.clone() {
            labels.get("app.kubernetes.io/component") == Some(&"frontend".to_owned())
        } else {
            false
        }
    }) {
        return kubewarden::reject_request(
            Some("No frontend component found".to_owned()),
            Some(404),
            None,
            None,
        );
    }

    // Check if the namespace has an authentication service deployed
    let kube_request = GetResourceRequest {
        api_version: "v1".to_owned(),
        kind: "Service".to_owned(),
        namespace: Some(namespace.clone()),
        name: "api-auth-service".to_owned(),
        disable_cache: false,
    };

    let service: Service = get_resource(&kube_request)?;
    if let Some(labels) = service.metadata().labels.clone() {
        if labels.get("app.kubernetes.io/part-of") != Some(&"api".to_owned()) {
            return kubewarden::reject_request(
                Some("No API authentication service found".to_owned()),
                Some(404),
                None,
                None,
            );
        }
    } else {
        return kubewarden::reject_request(
            Some("API authentication service must have labels".to_owned()),
            Some(404),
            None,
            None,
        );
    }
    if let Ok(manifest) = manifest("ghcr.io/kubewarden/tests/context-aware-test-policy:latest") {
        match manifest {
            kubewarden::host_capabilities::oci::OciManifestResponse::Image(_) => {}
            kubewarden::host_capabilities::oci::OciManifestResponse::ImageIndex(_) => {
                return kubewarden::reject_request(
                    Some("Invalid OCI manifest type. Got Image index manifest. But it should be image manifest".to_owned()),
                    Some(404),
                    None,
                    None,
                );
            }
        }
    } else {
        return kubewarden::reject_request(
            Some("cannot fetch policy manifest".to_owned()),
            Some(404),
            None,
            None,
        );
    }
    if let Ok(manifest) = manifest("ghcr.io/kubewarden/policy-server:latest") {
        match manifest {
            kubewarden::host_capabilities::oci::OciManifestResponse::Image(_) => {
                return kubewarden::reject_request(
                    Some("Invalid OCI manifest type. Got Image manifest. But it should be image index manifest".to_owned()),
                    Some(404),
                    None,
                    None,
                );
            }
            kubewarden::host_capabilities::oci::OciManifestResponse::ImageIndex(_) => {}
        }
    } else {
        return kubewarden::reject_request(
            Some("cannot fetch policy manifest".to_owned()),
            Some(404),
            None,
            None,
        );
    }

    kubewarden::accept_request()
}
