// Copyright 2023 Salesforce, Inc. All rights reserved.
#[macro_export]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!(T0);
        $name!(T0, T1);
        $name!(T0, T1, T2);
        $name!(T0, T1, T2, T3);
        $name!(T0, T1, T2, T3, T4);
        $name!(T0, T1, T2, T3, T4, T5);
        $name!(T0, T1, T2, T3, T4, T5, T6);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
    };
}
