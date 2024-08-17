pub mod spec;
use futures::StreamExt;
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Pod};
use kube::{
    api::{Patch, PatchParams},
    runtime::controller::{Action, Controller},
    Api, Client, ResourceExt,
};
use serde_json::{json, Value};
use spec::{CustomSpec, E5Spec, GemmaSpec, Main, MainSpec, QuantizedSpec};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {}
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
struct Data {
    /// kubernetes client
    client: Client,
    /// In memory state
    state: Arc<RwLock<()>>,
}

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
    let client = Client::try_default().await?;
    let pods = Api::<Main>::all(client.clone());

    let context = Arc::new(Data {
        client: client.clone(),
        state: Arc::new(RwLock::new(())),
    });

    Controller::new(pods.clone(), Default::default())
        .run(reconcile, error_policy, context)
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

// async fn reconcile<K, Ctx>(obj: Arc<K>, ctx: Arc<Ctx>) -> Result<Action> {
// where
//     K::DynamicType: Debug + Unpin,
//     ReconcilerFut: TryFuture<Ok = Action> + Send + 'static,
//     ReconcilerFut::Error: std::error::Error + Send + 'static,
//     // Bounds from impl:
//     K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
//     K::DynamicType: Eq + Hash + Clone {
//         todo!()
//     }

async fn reconcile(obj: Arc<Main>, ctx: Arc<Data>) -> Result<Action> {
    println!("reconcile request: {}", obj.name_any());
    let client = ctx.client.clone();
    let namespace = obj.namespace().unwrap();
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), &namespace);

    let worker_name = obj.metadata.clone().name.unwrap();
    let deployment_name = format!("{}-deployment", worker_name);

    let deployment = ActorsDeployments::Main(obj.spec.clone());
    let deployment = deployment.deployment(&worker_name, &deployment_name);
    deployments
        .patch(
            &deployment_name,
            &PatchParams::apply("onceuponai.dev"),
            &Patch::Apply(deployment),
        )
        .await
        .unwrap();

    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<Main>, _err: &Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(Duration::from_secs(5))
}

pub enum ActorsDeployments {
    Main(MainSpec),
    Custom(CustomSpec),
    Gemma(GemmaSpec),
    Quantized(QuantizedSpec),
    E5(E5Spec),
}

impl ActorsDeployments {
    pub fn deployment(&self, name: &str, deployment_name: &str) -> Value {
        match self {
            ActorsDeployments::Main(spec) => json!({
                "apiVersion": "apps/v1",
                "kind": "Deployment",
                "metadata": {
                    "name": deployment_name
                },
                "spec": {
                    "replicas": spec.replicas,
                    "selector": {
                        "matchLabels": {
                            "app": name
                        }
                    },
                    "template": {
                        "metadata": {
                            "labels": {
                                "app": name
                            }
                        },
                        "spec": {
                            "containers": [
                                {
                                    "name": name,
                                    "image": spec.image,
                                    "env": [
                                        {
                                            "name": "ACTOR_HOST",
                                            "value": spec.actor_host
                                        },
                                        {
                                            "name": "ACTOR_SEED",
                                            "value": spec.actor_seed
                                        }
                                    ],
                                    "ports": [
                                        {
                                            "containerPort": spec.server_port
                                        }
                                    ]
                                }
                            ]
                        }
                    }
                }
            }),

            ActorsDeployments::Custom(spec) => todo!(),
            ActorsDeployments::Gemma(spec) => todo!(),
            ActorsDeployments::Quantized(spec) => todo!(),
            ActorsDeployments::E5(spec) => todo!(),
        }
    }
}
