extern crate treexml;

error_chain!{
    links {
        XMLError(treexml::Error, treexml::ErrorKind);
    }

    errors {
        MissingDataInIPCMessage(channel: String, data: String) {
            description("missing data in IPC message")
            display("missing data {} in IPC channel {}", data, channel)
        }
        LogicError(desc: String) {
            description("logic error")
            display("logic error: {}", desc)
        }
        InvalidVariantInIPCChannel(channel: String, variant: String) {
            description("invalid variant in IPC channel")
            display("invalid variant {} in IPC channel {}", variant, channel)
        }
    }
}
