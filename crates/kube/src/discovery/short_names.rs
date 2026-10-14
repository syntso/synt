const SHORT_NAMES: [(&str, &str); 26] = [
    ("cm", "configmaps"),
    ("cj", "cronjobs"),
    ("crd", "customresourcedefinitions"),
    ("crds", "customresourcedefinitions"),
    ("csr", "certificatesigningrequests"),
    ("deploy", "deployments"),
    ("ds", "daemonsets"),
    ("ep", "endpoints"),
    ("ev", "events"),
    ("hpa", "horizontalpodautoscalers"),
    ("ing", "ingresses"),
    ("limits", "limitranges"),
    ("netpol", "networkpolicies"),
    ("no", "nodes"),
    ("ns", "namespaces"),
    ("pc", "priorityclasses"),
    ("pdb", "poddisruptionbudgets"),
    ("po", "pods"),
    ("pv", "persistentvolumes"),
    ("pvc", "persistentvolumeclaims"),
    ("quota", "resourcequotas"),
    ("rs", "replicasets"),
    ("sa", "serviceaccounts"),
    ("sc", "storageclasses"),
    ("sts", "statefulsets"),
    ("svc", "services"),
];

pub(super) fn plural_of(short: &str) -> Option<&'static str> {
    SHORT_NAMES
        .iter()
        .find(|(name, _)| *name == short)
        .map(|(_, plural)| *plural)
}
