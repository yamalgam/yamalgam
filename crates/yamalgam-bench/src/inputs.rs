//! Deterministic YAML input generators for benchmarks.
//!
//! All generators are pure functions producing identical output on every call.
//! No PRNG needed — patterns are arithmetic.

/// Kubernetes-style Deployment manifest (~2KB). Static, hand-written.
#[must_use]
pub fn kubernetes_deployment() -> String {
    r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: web-frontend
  namespace: production
  labels:
    app: web-frontend
    tier: frontend
    environment: production
    version: v2.4.1
  annotations:
    deployment.kubernetes.io/revision: "12"
    kubectl.kubernetes.io/last-applied-configuration: |
      {"apiVersion":"apps/v1","kind":"Deployment"}
spec:
  replicas: 3
  revisionHistoryLimit: 10
  selector:
    matchLabels:
      app: web-frontend
      tier: frontend
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: web-frontend
        tier: frontend
        version: v2.4.1
    spec:
      serviceAccountName: web-frontend
      terminationGracePeriodSeconds: 30
      containers:
        - name: web
          image: registry.example.com/web-frontend:v2.4.1
          imagePullPolicy: IfNotPresent
          ports:
            - name: http
              containerPort: 8080
              protocol: TCP
            - name: metrics
              containerPort: 9090
              protocol: TCP
          env:
            - name: NODE_ENV
              value: production
            - name: LOG_LEVEL
              value: info
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: db-credentials
                  key: url
            - name: REDIS_HOST
              valueFrom:
                configMapKeyRef:
                  name: redis-config
                  key: host
            - name: POD_NAME
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
          resources:
            requests:
              cpu: 250m
              memory: 256Mi
            limits:
              cpu: "1"
              memory: 512Mi
          livenessProbe:
            httpGet:
              path: /healthz
              port: http
            initialDelaySeconds: 15
            periodSeconds: 10
            timeoutSeconds: 3
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /ready
              port: http
            initialDelaySeconds: 5
            periodSeconds: 5
            timeoutSeconds: 2
            successThreshold: 1
          volumeMounts:
            - name: config
              mountPath: /etc/app/config
              readOnly: true
            - name: tmp
              mountPath: /tmp
        - name: sidecar
          image: registry.example.com/log-collector:v1.2.0
          resources:
            requests:
              cpu: 50m
              memory: 64Mi
            limits:
              cpu: 100m
              memory: 128Mi
          volumeMounts:
            - name: tmp
              mountPath: /tmp
              readOnly: true
      volumes:
        - name: config
          configMap:
            name: web-frontend-config
        - name: tmp
          emptyDir:
            sizeLimit: 100Mi
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchExpressions:
                    - key: app
                      operator: In
                      values:
                        - web-frontend
                topologyKey: kubernetes.io/hostname
"#
    .to_owned()
}

/// N records with varied field types (deterministic, no PRNG).
#[must_use]
pub fn records(n: usize) -> String {
    let mut out = String::with_capacity(n * 120);
    out.push_str("records:\n");
    for i in 0..n {
        out.push_str(&format!("  - id: {i}\n"));
        out.push_str(&format!("    name: \"record-{i}\"\n"));
        out.push_str(&format!("    value: {}.{}\n", i * 17 % 1000, i * 31 % 100));
        out.push_str(&format!("    active: {}\n", i % 2 == 0));
        out.push_str("    tags:\n");
        out.push_str(&format!("      - tag-{}\n", i % 50));
        out.push_str(&format!("      - tag-{}\n", (i * 7) % 50));
    }
    out
}

/// Deeply nested block mapping (depth levels).
#[must_use]
pub fn nested(depth: usize) -> String {
    let mut out = String::with_capacity(depth * 30);
    for d in 0..depth {
        let indent = "  ".repeat(d);
        out.push_str(&format!("{indent}level_{d}:\n"));
    }
    // Leaf value at deepest level.
    let indent = "  ".repeat(depth);
    out.push_str(&format!("{indent}value: \"leaf\"\n"));
    out
}

/// Many small 2-field objects in a sequence.
#[must_use]
pub fn small_objects(n: usize) -> String {
    let mut out = String::with_capacity(n * 40);
    out.push_str("items:\n");
    for i in 0..n {
        out.push_str(&format!("  - k: {i}\n"));
        out.push_str(&format!("    v: val-{i}\n"));
    }
    out
}

/// Single large literal block scalar.
#[must_use]
pub fn large_scalar(bytes: usize) -> String {
    let line = "The quick brown fox jumps over the lazy dog. ";
    let line_len = line.len();
    let header = "content: |\n";
    let mut out = String::with_capacity(bytes + header.len() + 128);
    out.push_str(header);
    let mut written = 0;
    while written < bytes {
        out.push_str("  ");
        out.push_str(line);
        out.push('\n');
        written += line_len + 3; // 2 indent + newline
    }
    out
}
