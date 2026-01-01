# Serialization Migration Benchmark: Flatbuffers vs. Bitcode

## 1. System Specifications
**Objective:** Profile the CPU and Bandwidth trade-offs of migrating the Game Server Protocol.

*   **Date:** 2026-XX-XX
*   **Test Environment:** Kubernetes (Production-like)
*   **Node Hardware:** Minisforum MS-01 Intel Core i5-12600H
*   **Container Resources:**
    *   **Request:** 1.0 CPU / 1Gi Mem
    *   **Limit:** 1.0 CPU / 1Gi Mem
*   **Used Tools:** Simulator Client, Tracy profiler, Prometheus

## 2. Methodology
We are comparing two fundamentally different approaches:
1.  **Flatbuffers:** Zero-copy access. "Deserialization" is effectively free (pointer casting), but accessing fields has a small overhead.
2.  **Bitcode:** Bit-packing. High upfront cost to deserialize into a Rust struct, but subsequent field access is native/free.

We'll run the simulator for 1 minute multiple times, using a different number of clients each run.

## 3. Results Summary

### Metric 1: Payload Size (Bandwidth)
*Lower is better. This is the primary motivation for the migration.*

| Flatbuffers (Bytes) | Bitcode (Bytes) | Reduction % |
| :--- | :--- | :--- |
| 32 B | 4 B | **-87.5%** |

### Metric 2: CPU Cost (Latency)

| Operation | Flatbuffers (ns) | Bitcode (ns) | Impact |
| :--- | :--- | :--- | :--- |
| **Serialize** | 150 ns | 800 ns | **5.3x Slower** |
| **Deserialize** | 0 ns* | 600 ns | **Infinite** |

> *Note: Flatbuffers "Deserialize" is 0ns because it is zero-copy. The cost is paid during field access.*

---

## 4. Critical Analysis

### Bandwidth vs. CPU Trade-off
The migration to Bitcode saves approximately **60% bandwidth** across the board.
*   **Cost:** The CPU cost increases by roughly **6x to 8x** for serialization.
*   **Context:** Our current Tick budget is 16ms (16,666 µs).
*   **Impact:** Even with the 8x increase, serialization takes `15µs` per tick, which is `0.09%` of the frame budget.

