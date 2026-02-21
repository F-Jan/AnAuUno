use std::sync::Arc;
use crate::connection::ConnectionContext;
use crate::message::Message;

pub struct Data<T: ?Sized>(Arc<T>);

impl<T> Data<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(data))
    }
}

impl<T: ?Sized> Clone for Data<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}


pub struct MessageRequest {
    pub message: Message,
    pub context: ConnectionContext,
}


pub trait ServiceMessageHandlerArg {
    fn from_message_request(request: &MessageRequest) -> Self;
}

impl ServiceMessageHandlerArg for () {
    fn from_message_request(_request: &MessageRequest) -> Self {
        ()
    }
}

impl<T> ServiceMessageHandlerArg for Data<T> {
    fn from_message_request(request: &MessageRequest) -> Self {
        todo!()
    }
}

impl ServiceMessageHandlerArg for Message {
    fn from_message_request(request: &MessageRequest) -> Self {
        request.message.clone()
    }
}

#[derive(Clone)]
pub struct Proto<T: protobuf::Message>(pub T);

impl<T: protobuf::Message> ServiceMessageHandlerArg for Proto<T> {
    fn from_message_request(request: &MessageRequest) -> Self {
        Proto(request.message.to_protobuf_message::<T>())
    }
}


macro_rules! factory_handler_arg_tuple {
    ( $($param:ident),+ $(,)? ) => {
        impl<$($param,)+> ServiceMessageHandlerArg for ($($param,)+)
        where
            $($param: ServiceMessageHandlerArg,)+
        {
            #[inline]
            fn from_message_request(request: &MessageRequest) -> Self {
                ($($param::from_message_request(request),)+)
            }
        }
    };
}

factory_handler_arg_tuple!(A);
factory_handler_arg_tuple!(A, B);
factory_handler_arg_tuple!(A, B, C);
factory_handler_arg_tuple!(A, B, C, D);
factory_handler_arg_tuple!(A, B, C, D, E);
factory_handler_arg_tuple!(A, B, C, D, E, F);
factory_handler_arg_tuple!(A, B, C, D, E, F, G);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J, K);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
factory_handler_arg_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);



pub trait ServiceMessageHandler<Args> {
    fn call(&self, args: Args);
}

macro_rules! factory_tuple ({ $($param:ident)* } => {
    impl<Func, $($param,)*> ServiceMessageHandler<($($param,)*)> for Func
    where
        Func: Fn($($param),*),
    {
        #[inline]
        #[allow(non_snake_case)]
        fn call(&self, ($($param,)*): ($($param,)*)) {
            (self)($($param,)*)
        }
    }
});

factory_tuple! {}
factory_tuple! { A }
factory_tuple! { A B }
factory_tuple! { A B C }
factory_tuple! { A B C D }
factory_tuple! { A B C D E }
factory_tuple! { A B C D E F }
factory_tuple! { A B C D E F G }
factory_tuple! { A B C D E F G H }
factory_tuple! { A B C D E F G H I }
factory_tuple! { A B C D E F G H I J }
factory_tuple! { A B C D E F G H I J K }
factory_tuple! { A B C D E F G H I J K L }
factory_tuple! { A B C D E F G H I J K L M }
factory_tuple! { A B C D E F G H I J K L M N }
factory_tuple! { A B C D E F G H I J K L M N O }
factory_tuple! { A B C D E F G H I J K L M N O P }



pub trait Service {

    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service;

}

pub struct MediaSinkServiceConfig {}

pub struct MediaSinkService {
    config: MediaSinkServiceConfig,
}

impl MediaSinkService {
    pub fn new(config: MediaSinkServiceConfig) -> Self {
        Self { config }
    }
}

impl Service for MediaSinkService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

        let media_sink = crate::protobuf::control::service::MediaSinkService::new();

        service
    }
}
