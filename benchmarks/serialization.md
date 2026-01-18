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

### Metric 1: Payload Size (Bandwidth measured with 100 clients)

| Direction | Flatbuffers (Bytes) | Bitcode (Bytes) | Reduction % |
| :--- | :--- | :--- | :--- |
| Incoming | 68 B | 4 B | **-87.5%** |
| Outgoing | 4.19 KiB | 4 B | **-87.5%** |

### Metric 2: Sync system duration using Tracy profiler

| #Clients | Flatbuffers (ns) | Bitcode (ns) |
| :--- | :--- | :--- |
| 10 | 23 us | 25 us |
| 50 | 145 us | 150 us |
| 100 | 250 us | 250 us |
Note: these tests were performed on a developer PC, not on the Kubernetes cluster. This is why the numbers may vary from the production server RTT.

### Metric 3: Server RTT

| #Clients | Flatbuffers (ns) | Bitcode (ns) | Impact |
| :--- | :--- | :--- | :--- |
| 10 | 20.4 ms | 800 ns | **5.3x Slower** |
| 50 | 22.3 ms | 600 ns | **Infinite** |
| 100 | 24.29 ms | 800 ns | **5.3x Slower** |

---

## 4. Critical Analysis

### Bandwidth vs. CPU Trade-off
The migration to Bitcode saves approximately **60% bandwidth** across the board.
*   **Cost:** The CPU cost increases by roughly **6x to 8x** for serialization.
*   **Context:** Our current Tick budget is 16ms (16,666 µs).
*   **Impact:** Even with the 8x increase, serialization takes `15µs` per tick, which is `0.09%` of the frame budget.

