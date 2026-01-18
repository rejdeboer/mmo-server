# Serialization

We chose to use Bitcode as the serialization tooling. Originally, we were using FlatBuffers, but after a while the schema became cumbersome to maintain. After testing Bitcode, we found that we were able to greatly simplify the networking code, while greatly reducing to bandwith usage and with negligible impact on serialization performance.
