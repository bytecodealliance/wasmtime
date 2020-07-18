/// Define a `wast::parser::Parse` implementation for an operator type.
#[macro_export]
macro_rules! define_parse_impl_for_operator {
    (
        $operator:ident {
            $(
                $keyword:ident => $variant:ident;
            )*
        }
    ) => {
        impl<'a> wast::parser::Parse<'a> for $operator {
            fn parse(p: wast::parser::Parser<'a>) -> wast::parser::Result<$operator> {
                /// Token definitions for our `Opcode` keywords.
                mod tok {
                    $(
                        wast::custom_keyword!($keyword);
                    )*
                }

                // Peek at the next token, and if it is the variant's
                // keyword, then consume it with `parse`, and finally return
                // the `Opcode` variant.
                $(
                    if p.peek::<tok::$keyword>() {
                        p.parse::<tok::$keyword>()?;
                        return Ok(Self::$variant);
                    }
                )*

                // If none of the keywords matched, then we get a parse error.
                Err(p.error(concat!("expected `", stringify!($operator), "`")))
            }
        }
    }
}

/// Define a `peepmatic_traits::TypingRules` implementation for the given
/// operator type.
#[macro_export]
macro_rules! define_typing_rules_impl_for_operator {
    (
        $operator:ident {
            $(
                $variant:ident {
                    $( immediates( $($immediate:ident),* ); )?
                    $( parameters( $($parameter:ident),* ); )?
                    result( $result:ident );
                    $( is_reduce($is_reduce:expr); )?
                    $( is_extend($is_extend:expr); )?
                }
            )*
        }
    ) => {
        impl $crate::TypingRules for $operator {
            fn result_type<'a, C>(
                &self,
                span: C::Span,
                typing_context: &mut C,
            ) -> C::TypeVariable
            where
                C: $crate::TypingContext<'a> {
                match self {
                    $(
                        Self::$variant => typing_context.$result(span),
                    )*

                    #[allow(dead_code)]
                    _ => $crate::unsupported("no typing rules defined for variant"),
                }
            }

            fn immediates_arity(&self) -> u8 {
                match self {
                    $(
                        Self::$variant => $crate::define_typing_rules_impl_for_operator!(
                            @arity;
                            $( $( $immediate, )* )?
                        ),
                    )*

                    #[allow(dead_code)]
                    _ => $crate::unsupported("no typing rules defined for variant"),
                }
            }

            fn immediate_types<'a, C>(
                &self,
                span: C::Span,
                typing_context: &mut C,
                types: &mut impl Extend<C::TypeVariable>,
            )
            where
                C: $crate::TypingContext<'a>
            {
                match self {
                    $(
                        Self::$variant => types.extend(
                            None.into_iter()
                                $(
                                    $(
                                        .chain(Some(typing_context.$immediate(span)))
                                    )*
                                )?
                        ),
                    )*

                    #[allow(dead_code)]
                    _ => $crate::unsupported("no typing rules defined for variant"),
                }
            }

            fn parameters_arity(&self) -> u8 {
                match self {
                    $(
                        Self::$variant => $crate::define_typing_rules_impl_for_operator!(
                            @arity;
                            $( $( $parameter, )* )?
                        ),
                    )*

                    #[allow(dead_code)]
                    _ => $crate::unsupported("no typing rules defined for variant"),
                }
            }

            fn parameter_types<'a, C>(
                &self,
                span: C::Span,
                typing_context: &mut C,
                types: &mut impl Extend<C::TypeVariable>,
            )
            where
                C: $crate::TypingContext<'a>
            {
                match self {
                    $(
                        Self::$variant => types.extend(
                            None.into_iter()
                                $(
                                    $(
                                        .chain(Some(typing_context.$parameter(span)))
                                    )*
                                )?
                        ),
                    )*

                    #[allow(dead_code)]
                    _ => $crate::unsupported("no typing rules defined for variant"),
                }
            }

            fn is_reduce(&self) -> bool {
                match self {
                    $(
                        Self::$variant if false $( || $is_reduce )? => false $( || $is_reduce )?,
                    )*
                    _ => false,
                }
            }

            fn is_extend(&self) -> bool {
                match self {
                    $(
                        Self::$variant if false $( || $is_extend )? => false $( || $is_extend )?,
                    )*
                    _ => false,
                }
            }
        }
    };

    // Base case: zero arity.
    (
        @arity;
    ) => {
        0
    };

    // Recursive case: count one for the head and add that to the arity of the
    // rest.
    (
        @arity;
        $head:ident,
        $( $rest:ident, )*
    ) => {
        1 + $crate::define_typing_rules_impl_for_operator!(
            @arity;
            $( $rest, )*
        )
    }
}

/// Define both a `wast::parser::Parse` implementation and a
/// `peepmatic_traits::TypingRules` implementation for the given operator type.
#[macro_export]
macro_rules! define_parse_and_typing_rules_for_operator {
    (
        $operator:ident {
            $(
                $keyword:ident => $variant:ident {
                    $( immediates( $($immediate:ident),* ); )?
                    $( parameters( $($parameter:ident),* ); )?
                    result( $result:ident );
                    $( is_reduce($is_reduce:expr); )?
                    $( is_extend($is_extend:expr); )?
                }
            )*
        }
        $( parse_cfg($parse_cfg:meta); )?
    ) => {
        $( #[cfg($parse_cfg)] )?
        $crate::define_parse_impl_for_operator! {
            $operator {
                $(
                    $keyword => $variant;
                )*
            }
        }

        $crate::define_typing_rules_impl_for_operator! {
            $operator {
                $(
                    $variant {
                        $( immediates( $($immediate),* ); )?
                        $( parameters( $($parameter),* ); )?
                        result( $result );
                        $( is_reduce($is_reduce); )?
                        $( is_extend($is_extend); )?
                    }
                )*
            }
        }
    }
}

/// Define an operator type, as well as its parsing and typing rules.
#[macro_export]
macro_rules! define_operator {
    (
        $( #[$attr:meta] )*
        $operator:ident {
            $(
                $keywrord:ident => $variant:ident {
                    $( immediates( $($immediate:ident),* ); )?
                    $( parameters( $($parameter:ident),* ); )?
                    result( $result:ident );
                    $( is_reduce($is_reduce:expr); )?
                    $( is_extend($is_extend:expr); )?
                }
            )*
        }
        $( parse_cfg($parse_cfg:meta); )?
    ) => {
        $( #[$attr] )*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[repr(u32)]
        pub enum $operator {
            $(
                $variant,
            )*
        }

        impl From<$operator> for u32 {
            #[inline]
            fn from(x: $operator) -> u32 {
                x as u32
            }
        }

        impl From<$operator> for core::num::NonZeroU32 {
            #[inline]
            fn from(x: $operator) -> core::num::NonZeroU32 {
                let x: u32 = x.into();
                core::num::NonZeroU32::new(x.checked_add(1).unwrap()).unwrap()
            }
        }

        impl core::convert::TryFrom<u32> for $operator {
            type Error = ();

            #[inline]
            fn try_from(x: u32) -> Result<Self, ()> {
                match x {
                    $(
                        x if x == Self::$variant.into() => Ok(Self::$variant),
                    )*
                    _ => Err(())
                }
            }
        }

        impl core::convert::TryFrom<core::num::NonZeroU32> for $operator {
            type Error = ();

            #[inline]
            fn try_from(x: core::num::NonZeroU32) -> Result<Self, ()> {
                let x = x.get().checked_sub(1).ok_or(())?;
                Self::try_from(x)
            }
        }

        $crate::define_parse_and_typing_rules_for_operator! {
            $operator {
                $(
                    $keywrord => $variant {
                        $( immediates( $($immediate),* ); )?
                        $( parameters( $($parameter),* ); )?
                        result( $result );
                        $( is_reduce($is_reduce); )?
                        $( is_extend($is_extend); )?
                    }
                )*
            }
            $( parse_cfg($parse_cfg); )?
        }
    }
}
