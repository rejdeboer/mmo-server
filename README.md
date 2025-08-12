# Modern MMORPG (Work in progress)

This is a high-performance, scalable backend for a massively multiplayer online game, built from the ground up in **Rust**. It serves as a personal exploration into modern game server architecture, leveraging an Entity Component System (ECS) with **Bevy**, asynchronous networking with **Tokio**, and a data-oriented design for a robust and performant foundation.

---

## Key Features & Technical Highlights

*   **Asynchronous & Non-Blocking:** Built on the **Tokio** runtime for massively concurrent network I/O, capable of handling thousands of simultaneous connections.
*   **Data-Oriented ECS Architecture:** Uses the **Bevy** game engine as a powerful, data-oriented Entity Component System on the server-side for clean and scalable state management.
*   **Dual-Protocol Networking:**
    *   **UDP (via `bevy_renet`):** Handles high-frequency, unreliable game state updates like player movement and animations, crucial for a responsive real-time experience.
    *   **WebSockets (via `tungstenite`):** Manages a separate, reliable social server for features like chat, guilds, and presence, demonstrating a microservice-oriented approach.
*   **High-Performance Serialization:** Utilizes **FlatBuffers** for zero-copy deserialization of network packets, minimizing server-side CPU load and memory allocations under high traffic.
*   **Interest Management System:** A custom, grid-based visibility system efficiently determines which clients receive which entity updates, dramatically reducing network bandwidth and ensuring scalability.
*   **Persistent World:** Player and world state is persisted in a **PostgreSQL** database, with asynchronous database calls handled by **`sqlx`** to ensure the main game loop is never blocked by I/O.
*   **Ready for Deployment:** The entire application is containerized with **Docker** and orchestrated with **Kubernetes**, including YAML manifests for a full deployment on a bare-metal homelab cluster.


