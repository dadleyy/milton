module Button exposing (..)

import Html
import Html.Attributes as AT
import Html.Events as EV


type ButtonIcon
    = LightOn
    | LightOff


type Button a
    = Icon ButtonIcon a
    | Text String a


type ButtonVariant
    = Disabled
    | Primary
    | Secondary
    | Warning


view : ( Button a, ButtonVariant ) -> Html.Html a
view kind =
    let
        isDisabled =
            Tuple.second kind == Disabled

        variantClass =
            case Tuple.second kind of
                Primary ->
                    "button-primary"

                Secondary ->
                    "button-primary"

                Warning ->
                    "button-warning"

                Disabled ->
                    "button-disabled"
    in
    Html.div []
        [ case Tuple.first kind of
            Text content message ->
                Html.button
                    [ AT.class variantClass
                    , EV.onClick message
                    , AT.disabled isDisabled
                    ]
                    [ Html.text content ]

            Icon ico message ->
                Html.button
                    [ AT.class ("icon-button " ++ variantClass)
                    , EV.onClick message
                    , AT.disabled isDisabled
                    ]
                    [ Html.i [ AT.class (iconClass ico) ] [] ]
        ]


iconClass : ButtonIcon -> String
iconClass kind =
    case kind of
        LightOn ->
            "fa-solid fa-lightbulb"

        LightOff ->
            "fa-regular fa-lightbulb"
