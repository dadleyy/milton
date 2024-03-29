module Main exposing (..)

import Browser
import Browser.Navigation as Nav
import Button
import Html
import Html.Attributes as AT
import Http
import Json.Decode as JD
import Json.Encode as JE
import Task
import Time
import Url


type alias Environment =
    { apiRoot : String
    , uiRoot : String
    , loginURL : String
    , logoutURL : String
    , version : String
    }


type alias SessionUserData =
    { name : String
    , picture : String
    , user_id : String
    }


type alias SessionData =
    { user : Maybe SessionUserData }


type alias SessionPayload =
    { ok : Bool
    , session : SessionData
    }


type Request
    = Done (Result Http.Error ())
    | Pending
    | NotAsked


type ImageMode
    = Stream
    | Snapshot


type alias HomePage =
    { lastRequest : Request
    , imageMode : ImageMode
    }


type Page
    = Home HomePage
    | Input String


type LightRequest
    = State Bool
    | Color Button.RGBColor


type Model
    = Booting Environment
    | Unauthorized Environment
    | Authorized Page Environment SessionUserData
    | Failed Environment


type Message
    = LinkClicked Browser.UrlRequest
    | UrlChanged Url.Url
    | ToggleLight LightRequest
    | ToggleImageMode
    | SessionLoaded (Result Http.Error SessionPayload)
    | CommandResponse (Result Http.Error ())
    | Tick Time.Posix


main : Program Environment Model Message
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }


init : Environment -> Url.Url -> Nav.Key -> ( Model, Cmd Message )
init env url key =
    let
        model =
            Booting env
    in
    ( model, loadAuth model )


update : Message -> Model -> ( Model, Cmd Message )
update message model =
    let
        env =
            envFromModel model
    in
    case message of
        Tick _ ->
            ( model, Cmd.none )

        ToggleImageMode ->
            case model of
                Authorized (Home homeState) _ session ->
                    let
                        newMode =
                            case homeState.imageMode of
                                Stream ->
                                    Snapshot

                                Snapshot ->
                                    Stream
                    in
                    ( Authorized (Home (HomePage homeState.lastRequest newMode)) env session, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        CommandResponse result ->
            case model of
                Authorized (Home homeState) _ session ->
                    ( Authorized (Home (HomePage (Done result) homeState.imageMode)) env session, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        SessionLoaded (Ok payload) ->
            modelFromSessionPayload env payload

        SessionLoaded (Err error) ->
            ( Failed env, Cmd.none )

        LinkClicked (Browser.Internal url) ->
            ( model, Cmd.none )

        LinkClicked (Browser.External href) ->
            ( model, Nav.load href )

        ToggleLight lightState ->
            case model of
                Authorized (Home homeState) _ session ->
                    case homeState.lastRequest of
                        Pending ->
                            ( model, Cmd.none )

                        _ ->
                            ( Authorized (Home (HomePage Pending homeState.imageMode)) env session, makeLightRequest env lightState )

                _ ->
                    ( model, Cmd.none )

        UrlChanged _ ->
            ( model, Cmd.none )


view : Model -> Browser.Document Message
view model =
    { title = "milton-ui"
    , body = [ viewBody model, viewFooter model ]
    }


subscriptions : Model -> Sub Message
subscriptions model =
    Time.every 1000 Tick


envFromModel : Model -> Environment
envFromModel model =
    case model of
        Authorized _ env _ ->
            env

        Failed env ->
            env

        Booting env ->
            env

        Unauthorized env ->
            env


modelFromSessionPayload : Environment -> SessionPayload -> ( Model, Cmd Message )
modelFromSessionPayload env payload =
    case payload.ok of
        True ->
            ( Maybe.map (Authorized (Home (HomePage NotAsked Stream)) env) payload.session.user
                |> Maybe.withDefault (Unauthorized env)
            , Cmd.none
            )

        False ->
            ( Unauthorized env, Cmd.none )


getAuthURL : Model -> String
getAuthURL model =
    (envFromModel model |> .apiRoot) ++ "/auth/identify"


loadAuth : Model -> Cmd Message
loadAuth model =
    Http.get { url = getAuthURL model, expect = Http.expectJson SessionLoaded sessionDecoder }


sessionUserDataDecoder : JD.Decoder SessionUserData
sessionUserDataDecoder =
    JD.map3 SessionUserData
        (JD.field "name" JD.string)
        (JD.field "picture" JD.string)
        (JD.field "user_id" JD.string)


sessionFieldDecoder : JD.Decoder SessionData
sessionFieldDecoder =
    JD.map SessionData (JD.nullable (JD.field "user" sessionUserDataDecoder))


sessionDecoder : JD.Decoder SessionPayload
sessionDecoder =
    JD.map2 SessionPayload
        (JD.field "ok" JD.bool)
        (JD.field "session" sessionFieldDecoder)


viewFooter : Model -> Html.Html Message
viewFooter model =
    Html.footer [ AT.class "footer fixed bottom-0 left-0 w-full bg-slate-800" ]
        [ Html.div [ AT.class "flex items-center px-3 py-2 border-t border-solid border-slate-800" ]
            [ Html.div [ AT.class "ml-auto" ]
                [ Html.a [ AT.href "https://github.com/dadleyy/milton", AT.rel "noopener", AT.target "_blank" ]
                    [ Html.text (envFromModel model |> .version) ]
                ]
            ]
        ]


viewBody : Model -> Html.Html Message
viewBody model =
    Html.div [ AT.class "w-full h-full relative pb-12" ]
        [ case model of
            Booting _ ->
                Html.div [ AT.class "relative w-full h-full flex items-center" ]
                    [ Html.div [ AT.class "mx-auto" ] [ Html.text "loading..." ]
                    ]

            Unauthorized _ ->
                Html.div [ AT.class "relative w-full h-full flex items-center" ]
                    [ Html.div [ AT.class "mx-auto" ]
                        [ Html.a
                            [ AT.href (envFromModel model |> .loginURL)
                            , AT.target "_self"
                            , AT.rel "noopener"
                            ]
                            [ Html.text "login" ]
                        ]
                    ]

            Authorized activePage env session ->
                Html.div [] [ header env session, viewPage activePage env session ]

            Failed _ ->
                Html.div [] [ Html.text "unable to load." ]
        ]


viewPage : Page -> Environment -> SessionUserData -> Html.Html Message
viewPage page env session =
    case page of
        Home homePage ->
            let
                isBusy =
                    case homePage.lastRequest of
                        Pending ->
                            True

                        _ ->
                            False
            in
            Html.div [ AT.class "pt-8 flex items-center flex-col w-full h-full" ]
                [ Html.div [ AT.class "mx-auto" ]
                    [ imageControl env homePage ]
                , Html.div [ AT.class "mx-auto flex items-center mt-4" ]
                    [ Html.div [ AT.class "mr-1" ]
                        [ Button.view
                            ( Button.Icon Button.LightOn (ToggleLight (State True))
                            , if isBusy then
                                Button.Disabled

                              else
                                Button.Primary
                            )
                        ]
                    , Html.div [ AT.class "mr-1" ]
                        [ Button.view
                            ( Button.Icon Button.LightOff (ToggleLight (State False))
                            , if isBusy then
                                Button.Disabled

                              else
                                Button.Warning
                            )
                        ]
                    , Html.div [ AT.class "mr-1" ]
                        [ Button.view
                            ( Button.Icon Button.CircleDot (ToggleLight (Color Button.Red))
                            , if isBusy then
                                Button.Disabled

                              else
                                Button.RGB Button.Red
                            )
                        ]
                    , Html.div [ AT.class "mr-1" ]
                        [ Button.view
                            ( Button.Icon Button.CircleDot (ToggleLight (Color Button.Green))
                            , if isBusy then
                                Button.Disabled

                              else
                                Button.RGB Button.Green
                            )
                        ]
                    , Html.div []
                        [ Button.view
                            ( Button.Icon Button.CircleDot (ToggleLight (Color Button.Blue))
                            , if isBusy then
                                Button.Disabled

                              else
                                Button.RGB Button.Blue
                            )
                        ]
                    ]
                ]

        Input value ->
            Html.div [ AT.class "px-3 py-3" ] []


header : Environment -> SessionUserData -> Html.Html Message
header env session =
    Html.div [ AT.class "px-3 py-3 flex items-center border-b border-solid border-stone-700" ]
        [ Html.div []
            [ Html.div [] [ Html.text session.name ] ]
        , Html.div [ AT.class "ml-auto" ]
            [ Html.a [ AT.href (env |> .logoutURL), AT.target "_self", AT.rel "noopener" ]
                [ Html.text "logout" ]
            ]
        ]


viewButton : String -> Html.Html Message
viewButton message =
    Html.button [] [ Html.text "" ]


makeLightRequestBody : LightRequest -> JE.Value
makeLightRequestBody requestKind =
    case requestKind of
        State bool ->
            JE.object
                [ ( "kind", JE.string "state" )
                , ( "on", JE.bool bool )
                ]

        Color Button.Red ->
            JE.object
                [ ( "kind", JE.string "basic_color" )
                , ( "color", JE.string "red" )
                ]

        Color Button.Green ->
            JE.object
                [ ( "kind", JE.string "basic_color" )
                , ( "color", JE.string "green" )
                ]

        Color Button.Blue ->
            JE.object
                [ ( "kind", JE.string "basic_color" )
                , ( "color", JE.string "blue" )
                ]


makeLightRequest : Environment -> LightRequest -> Cmd Message
makeLightRequest env lightState =
    Http.post
        { body = Http.jsonBody (makeLightRequestBody lightState)
        , url = env.apiRoot ++ "/control"
        , expect = Http.expectWhatever CommandResponse
        }


imageControl : Environment -> HomePage -> Html.Html Message
imageControl env homePage =
    let
        buttonIcon =
            case homePage.imageMode of
                Stream ->
                    Button.Camera

                Snapshot ->
                    Button.Video
    in
    Html.div []
        [ Html.div [ AT.class "mx-auto text-center justify-center mb-2" ]
            [ Html.div [ AT.class "inline-block" ] [ Button.view ( Button.Icon buttonIcon ToggleImageMode, Button.Primary ) ] ]
        , case homePage.imageMode of
            Stream ->
                Html.img [ AT.src (env.apiRoot ++ "/control/video-stream") ] []

            Snapshot ->
                Html.img [ AT.src (env.apiRoot ++ "/control/video-snapshot") ] []
        ]
