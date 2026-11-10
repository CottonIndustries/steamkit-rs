# SteamKit2 — Rust port · Developer's guide

> A complete, behaviour-preserving port of the C# [SteamKit2](https://github.com/SteamRE/SteamKit)
> library to Rust. This guide explains how the port is put together, what every file does,
> and how to work on it safely.
>
> | | |
> |---|---|
> | C# source | `SteamKit2` 4.0.0-Beta.1, commit `94812dc3` (2026-11-02), **still pre-`ProtoClient`** — it uses `CMClient` + `AsyncJobManager` + `CallbackMgr`, *not* `ProtoClient`/`ProtoHandler` |
> | Valve protobuf source | commit `bdb3957d`, vendored at `vendor/Protobufs/` |
> | Crates | 6 · **219** library files · **67,096** hand-written lines (all crates, excluding the module manifests and the generated enums) |
> | Tests | **456 passed, 0 failed**; 4 ignored entries |
> | Unsafe | forbidden in the library (`#![forbid(unsafe_code)]`); only `steamkit-nethook` uses it |
> | Status | over **TCP** the port completes the whole CM flow and the server answers the logon; **a credentialed login (QR flow) succeeded on 2026-10-22**, and a logged-on session passed 9 of 9 live checks on 2026-10-23 |

---

## Table of contents

- [1. What this is](#1-what-this-is)
  - [1.1 The single most important thing to know](#11-the-single-most-important-thing-to-know)
  - [1.2 The standing constraints](#12-the-standing-constraints)
  - [1.3 The documents beside this one](#13-the-documents-beside-this-one)
- [2. Repository layout](#2-repository-layout)
  - [2.1 The crate dependency graph](#21-the-crate-dependency-graph)
- [3. The architecture](#3-the-architecture)
  - [3.1 The shape of the library](#31-the-shape-of-the-library)
  - [3.2 How a message flows](#32-how-a-message-flows)
  - [3.2.1 The one path that is not in that diagram: the CM server switch](#321-the-one-path-that-is-not-in-that-diagram-the-cm-server-switch)
  - [3.3 The async model](#33-the-async-model)
  - [3.4 The error model](#34-the-error-model)
  - [3.5 The protobuf strategy](#35-the-protobuf-strategy)
  - [3.6 The 91 Valve enums](#36-the-91-valve-enums)
  - [3.7 The wire-format layer](#37-the-wire-format-layer)
- [4. The C# → Rust mapping](#4-the-c--rust-mapping)
  - [4.1 Names that changed](#41-names-that-changed)
  - [4.2 Names that did **not** change](#42-names-that-did-not-change)
  - [4.3 Idioms that changed shape](#43-idioms-that-changed-shape)
  - [4.4 The full C# → Rust substitution table](#44-the-full-c--rust-substitution-table)
  - [4.5 The design decisions that were hard, and what they resolved to](#45-the-design-decisions-that-were-hard-and-what-they-resolved-to)
  - [4.6 The two seams the port invented](#46-the-two-seams-the-port-invented)
- [5. The library, file by file](#5-the-library-file-by-file)
  - [5.1 crates/steamkit/src/types/ — the value types](#51-cratessteamkitsrctypes--the-value-types)
  - [5.2 crates/steamkit/src/base/ — message framing and the wire format](#52-cratessteamkitsrcbase--message-framing-and-the-wire-format)
    - [crates/steamkit/src/types/ — directory overview](#cratessteamkitsrctypes--directory-overview)
    - [crates/steamkit/src/types/depot_manifest.rs — 644 lines](#cratessteamkitsrctypesdepot_manifestrs--644-lines)
    - [crates/steamkit/src/types/game_id.rs — 179 lines](#cratessteamkitsrctypesgame_idrs--179-lines)
    - [crates/steamkit/src/types/global_id.rs — 168 lines](#cratessteamkitsrctypesglobal_idrs--168-lines)
    - [crates/steamkit/src/types/job_id.rs — 515 lines](#cratessteamkitsrctypesjob_idrs--515-lines)
    - [crates/steamkit/src/types/key_value.rs — 1137 lines](#cratessteamkitsrctypeskey_valuers--1137-lines)
    - [crates/steamkit/src/types/manifest.rs — 306 lines](#cratessteamkitsrctypesmanifestrs--306-lines)
    - [crates/steamkit/src/types/message_object.rs — 52 lines](#cratessteamkitsrctypesmessage_objectrs--52-lines)
    - [crates/steamkit/src/types/pub_file.rs — 56 lines](#cratessteamkitsrctypespub_filers--56-lines)
    - [crates/steamkit/src/types/steam_id.rs — 820 lines](#cratessteamkitsrctypessteam_idrs--820-lines)
    - [crates/steamkit/src/base/ — directory overview](#cratessteamkitsrcbase--directory-overview)
    - [crates/steamkit/src/base/msg_base.rs — 479 lines](#cratessteamkitsrcbasemsg_basers--479-lines)
    - [crates/steamkit/src/base/client_msg.rs — 560 lines](#cratessteamkitsrcbaseclient_msgrs--560-lines)
    - [crates/steamkit/src/base/packet_base.rs — 217 lines](#cratessteamkitsrcbasepacket_basers--217-lines)
    - [crates/steamkit/src/base/gc/msg_base_gc.rs — 50 lines](#cratessteamkitsrcbasegcmsg_base_gcrs--50-lines)
    - [crates/steamkit/src/base/gc/client_msg_gc.rs — 299 lines](#cratessteamkitsrcbasegcclient_msg_gcrs--299-lines)
    - [crates/steamkit/src/base/gc/packet_base_gc.rs — 144 lines](#cratessteamkitsrcbasegcpacket_base_gcrs--144-lines)
    - [crates/steamkit/src/base/generated/steam_language_internal.rs — 1168 lines](#cratessteamkitsrcbasegeneratedsteam_language_internalrs--1168-lines)
    - [crates/steamkit/src/base/generated/steam_language.rs — 18785 lines](#cratessteamkitsrcbasegeneratedsteam_languagers--18785-lines)
  - [5.3 crates/steamkit/src/util/ — the helpers, and the vendored LZMA SDK](#53-cratessteamkitsrcutil--the-helpers-and-the-vendored-lzma-sdk)
    - [Conventions used throughout util/](#conventions-used-throughout-util)
    - [util/lzma/ — the vendored 7-Zip LZMA SDK (compress side only)](#utillzma--the-vendored-7-zip-lzma-sdk-compress-side-only)
    - [The split: util/lzma/ is the *compress* side only](#the-split-utillzma-is-the-compress-side-only)
    - [File reference](#file-reference)
    - [crates/steamkit/src/util/adler32.rs — 51 lines](#cratessteamkitsrcutiladler32rs--51-lines)
    - [crates/steamkit/src/util/crypto_helper.rs — 88 lines](#cratessteamkitsrcutilcrypto_helperrs--88-lines)
    - [crates/steamkit/src/util/date_utils.rs — 50 lines](#cratessteamkitsrcutildate_utilsrs--50-lines)
    - [crates/steamkit/src/util/debug_log.rs — 255 lines](#cratessteamkitsrcutildebug_logrs--255-lines)
    - [crates/steamkit/src/util/debug_network_listener.rs — 159 lines](#cratessteamkitsrcutildebug_network_listenerrs--159-lines)
    - [crates/steamkit/src/util/hardware_utils.rs — 618 lines](#cratessteamkitsrcutilhardware_utilsrs--618-lines)
    - [crates/steamkit/src/util/i_log_context.rs — 38 lines](#cratessteamkitsrcutili_log_contextrs--38-lines)
    - [crates/steamkit/src/util/key_dictionary.rs — 84 lines](#cratessteamkitsrcutilkey_dictionaryrs--84-lines)
    - [crates/steamkit/src/util/lzma_decoder.rs — 493 lines](#cratessteamkitsrcutillzma_decoderrs--493-lines)
    - [crates/steamkit/src/util/lzma_util.rs — 119 lines](#cratessteamkitsrcutillzma_utilrs--119-lines)
    - [crates/steamkit/src/util/mac_helpers.rs — 140 lines](#cratessteamkitsrcutilmac_helpersrs--140-lines)
    - [crates/steamkit/src/util/msg_util.rs — 93 lines](#cratessteamkitsrcutilmsg_utilrs--93-lines)
    - [crates/steamkit/src/util/net_helpers.rs — 106 lines](#cratessteamkitsrcutilnet_helpersrs--106-lines)
    - [crates/steamkit/src/util/scheduled_function.rs — 144 lines](#cratessteamkitsrcutilscheduled_functionrs--144-lines)
    - [crates/steamkit/src/util/simple_console_debug_listener.rs — 27 lines](#cratessteamkitsrcutilsimple_console_debug_listenerrs--27-lines)
    - [crates/steamkit/src/util/steam_kit_web_request_exception.rs — 119 lines](#cratessteamkitsrcutilsteam_kit_web_request_exceptionrs--119-lines)
    - [crates/steamkit/src/util/stream_helpers.rs — 222 lines](#cratessteamkitsrcutilstream_helpersrs--222-lines)
    - [crates/steamkit/src/util/task_extensions.rs — 81 lines](#cratessteamkitsrcutiltask_extensionsrs--81-lines)
    - [crates/steamkit/src/util/utils.rs — 234 lines](#cratessteamkitsrcutilutilsrs--234-lines)
    - [util/ — the decompression chain, the VZip delta, the stopwatch and the Windows helper](#util--the-decompression-chain-the-vzip-delta-the-stopwatch-and-the-windows-helper)
    - [crates/steamkit/src/util/value_stopwatch.rs — 38 lines](#cratessteamkitsrcutilvalue_stopwatchrs--38-lines)
    - [crates/steamkit/src/util/v_zip_delta_util.rs — 46 lines](#cratessteamkitsrcutilv_zip_delta_utilrs--46-lines)
    - [crates/steamkit/src/util/win32_helpers.rs — 79 lines](#cratessteamkitsrcutilwin32_helpersrs--79-lines)
    - [crates/steamkit/src/util/v_zstd_util.rs — 93 lines](#cratessteamkitsrcutilv_zstd_utilrs--93-lines)
    - [crates/steamkit/src/util/v_zip_util.rs — 172 lines](#cratessteamkitsrcutilv_zip_utilrs--172-lines)
    - [crates/steamkit/src/util/zip_util.rs — 196 lines](#cratessteamkitsrcutilzip_utilrs--196-lines)
    - [LZMA SDK — the compress side](#lzma-sdk--the-compress-side)
    - [The compress / decompress split](#the-compress--decompress-split)
    - [Files in this section](#files-in-this-section)
    - [crates/steamkit/src/util/lzma/i_coder.rs — 129 lines](#cratessteamkitsrcutillzmai_coderrs--129-lines)
    - [crates/steamkit/src/util/lzma/common/crc.rs — 87 lines](#cratessteamkitsrcutillzmacommoncrcrs--87-lines)
    - [crates/steamkit/src/util/lzma/compress/range_coder/range_coder.rs — 153 lines](#cratessteamkitsrcutillzmacompressrange_coderrange_coderrs--153-lines)
    - [crates/steamkit/src/util/lzma/compress/range_coder/range_coder_bit.rs — 109 lines](#cratessteamkitsrcutillzmacompressrange_coderrange_coder_bitrs--109-lines)
    - [crates/steamkit/src/util/lzma/compress/range_coder/range_coder_bit_tree.rs — 127 lines](#cratessteamkitsrcutillzmacompressrange_coderrange_coder_bit_treers--127-lines)
    - [crates/steamkit/src/util/lzma/compress/lz/i_match_finder.rs — 51 lines](#cratessteamkitsrcutillzmacompresslzi_match_finderrs--51-lines)
    - [crates/steamkit/src/util/lzma/compress/lz/lz_in_window.rs — 264 lines](#cratessteamkitsrcutillzmacompresslzlz_in_windowrs--264-lines)
    - [crates/steamkit/src/util/lzma/compress/lz/lz_bin_tree.rs — 591 lines](#cratessteamkitsrcutillzmacompresslzlz_bin_treers--591-lines)
    - [crates/steamkit/src/util/lzma/compress/lzma/lzma_base.rs — 142 lines](#cratessteamkitsrcutillzmacompresslzmalzma_basers--142-lines)
    - [crates/steamkit/src/util/lzma/compress/lzma/lzma_encoder.rs — 2130 lines](#cratessteamkitsrcutillzmacompresslzmalzma_encoderrs--2130-lines)
  - [5.4 The crate root, the error type, and the CM client](#54-the-crate-root-the-error-type-and-the-cm-client)
    - [crates/steamkit-tests/src/lib.rs — 1 line](#cratessteamkit-testssrclibrs--1-line)
    - [crates/steamkit-samples/src/lib.rs — 6 lines](#cratessteamkit-samplessrclibrs--6-lines)
    - [crates/steamkit/src/lib.rs — 33 lines](#cratessteamkitsrclibrs--33-lines)
    - [crates/steamkit/src/error.rs — 105 lines](#cratessteamkitsrcerrorrs--105-lines)
    - [crates/steamkit/src/steam/cm_client.rs — 1116 lines](#cratessteamkitsrcsteamcm_clientrs--1116-lines)
  - [5.5 crates/steamkit/src/networking/steam3/ — the Steam3 transports](#55-cratessteamkitsrcnetworkingsteam3--the-steam3-transports)
  - [5.6 crates/steamkit/src/steam/steam_client/ — the client, the jobs, the callbacks](#56-cratessteamkitsrcsteamsteam_client--the-client-the-jobs-the-callbacks)
    - [crates/steamkit/src/networking/steam3/ — the Steam3 transport layer](#cratessteamkitsrcnetworkingsteam3--the-steam3-transport-layer)
    - [crates/steamkit/src/networking/steam3/mod.rs — 12 lines](#cratessteamkitsrcnetworkingsteam3modrs--12-lines)
    - [crates/steamkit/src/networking/steam3/disconnected_event_args.rs — 15 lines](#cratessteamkitsrcnetworkingsteam3disconnected_event_argsrs--15-lines)
    - [crates/steamkit/src/networking/steam3/net_msg_event_args.rs — 27 lines](#cratessteamkitsrcnetworkingsteam3net_msg_event_argsrs--27-lines)
    - [crates/steamkit/src/networking/steam3/protocol_type.rs — 67 lines](#cratessteamkitsrcnetworkingsteam3protocol_typers--67-lines)
    - [crates/steamkit/src/networking/steam3/i_connection.rs — 200 lines](#cratessteamkitsrcnetworkingsteam3i_connectionrs--200-lines)
    - [crates/steamkit/src/networking/steam3/net_filter_encryption_with_hmac.rs — 271 lines](#cratessteamkitsrcnetworkingsteam3net_filter_encryption_with_hmacrs--271-lines)
    - [crates/steamkit/src/networking/steam3/envelope_encrypted_connection.rs — 489 lines](#cratessteamkitsrcnetworkingsteam3envelope_encrypted_connectionrs--489-lines)
    - [crates/steamkit/src/networking/steam3/proxy.rs — 479 lines](#cratessteamkitsrcnetworkingsteam3proxyrs--479-lines)
    - [crates/steamkit/src/networking/steam3/tcp_connection.rs — 323 lines](#cratessteamkitsrcnetworkingsteam3tcp_connectionrs--323-lines)
    - [crates/steamkit/src/networking/steam3/udp_packet.rs — 413 lines](#cratessteamkitsrcnetworkingsteam3udp_packetrs--413-lines)
    - [crates/steamkit/src/networking/steam3/udp_connection.rs — 727 lines](#cratessteamkitsrcnetworkingsteam3udp_connectionrs--727-lines)
    - [crates/steamkit/src/networking/steam3/web_socket_context.rs — 564 lines](#cratessteamkitsrcnetworkingsteam3web_socket_contextrs--564-lines)
    - [crates/steamkit/src/networking/steam3/web_socket_connection.rs — 229 lines](#cratessteamkitsrcnetworkingsteam3web_socket_connectionrs--229-lines)
    - [crates/steamkit/src/steam/steam_client/ — the client facade, jobs, callbacks and configuration](#cratessteamkitsrcsteamsteam_client--the-client-facade-jobs-callbacks-and-configuration)
    - [crates/steamkit/src/steam/steam_client/mod.rs — 6 lines](#cratessteamkitsrcsteamsteam_clientmodrs--6-lines)
    - [crates/steamkit/src/steam/steam_client/steam_client.rs — 725 lines](#cratessteamkitsrcsteamsteam_clientsteam_clientrs--725-lines)
    - [crates/steamkit/src/steam/steam_client/async_job_manager.rs — 513 lines](#cratessteamkitsrcsteamsteam_clientasync_job_managerrs--513-lines)
    - [crates/steamkit/src/steam/steam_client/async_job_failed_exception.rs — 74 lines](#cratessteamkitsrcsteamsteam_clientasync_job_failed_exceptionrs--74-lines)
    - [crates/steamkit/src/steam/steam_client/callback_mgr/mod.rs — 3 lines](#cratessteamkitsrcsteamsteam_clientcallback_mgrmodrs--3-lines)
    - [crates/steamkit/src/steam/steam_client/callback_mgr/callback_msg.rs — 74 lines](#cratessteamkitsrcsteamsteam_clientcallback_mgrcallback_msgrs--74-lines)
    - [crates/steamkit/src/steam/steam_client/callback_mgr/callback_mgr_internals.rs — 192 lines](#cratessteamkitsrcsteamsteam_clientcallback_mgrcallback_mgr_internalsrs--192-lines)
    - [crates/steamkit/src/steam/steam_client/callback_mgr/callback_mgr.rs — 325 lines](#cratessteamkitsrcsteamsteam_clientcallback_mgrcallback_mgrrs--325-lines)
    - [crates/steamkit/src/steam/steam_client/callbacks.rs — 83 lines](#cratessteamkitsrcsteamsteam_clientcallbacksrs--83-lines)
    - [crates/steamkit/src/steam/steam_client/configuration/mod.rs — 5 lines](#cratessteamkitsrcsteamsteam_clientconfigurationmodrs--5-lines)
    - [crates/steamkit/src/steam/steam_client/configuration/i_machine_info_provider.rs — 17 lines](#cratessteamkitsrcsteamsteam_clientconfigurationi_machine_info_providerrs--17-lines)
    - [crates/steamkit/src/steam/steam_client/configuration/i_steam_configuration_builder.rs — 49 lines](#cratessteamkitsrcsteamsteam_clientconfigurationi_steam_configuration_builderrs--49-lines)
    - [crates/steamkit/src/steam/steam_client/configuration/steam_configuration_state.rs — 139 lines](#cratessteamkitsrcsteamsteam_clientconfigurationsteam_configuration_staters--139-lines)
    - [crates/steamkit/src/steam/steam_client/configuration/steam_configuration_builder.rs — 112 lines](#cratessteamkitsrcsteamsteam_clientconfigurationsteam_configuration_builderrs--112-lines)
    - [crates/steamkit/src/steam/steam_client/configuration/steam_configuration.rs — 172 lines](#cratessteamkitsrcsteamsteam_clientconfigurationsteam_configurationrs--172-lines)
  - [5.7 crates/steamkit/src/steam/handlers/ — the 15 protocol handlers](#57-cratessteamkitsrcsteamhandlers--the-15-protocol-handlers)
    - [The handler pattern](#the-handler-pattern)
    - [The trait](#the-trait)
    - [How a handler declares which EMsg values it owns](#how-a-handler-declares-which-emsg-values-it-owns)
    - [Registration](#registration)
    - [The dispatch loop](#the-dispatch-loop)
    - [The two-part file convention](#the-two-part-file-convention)
    - [How a callback is subscribed to and dispatched](#how-a-callback-is-subscribed-to-and-dispatched)
    - [From a packet to a typed callback](#from-a-packet-to-a-typed-callback)
    - [The JobID correlation](#the-jobid-correlation)
    - [The C# class versus the Rust trait + struct pair](#the-c-class-versus-the-rust-trait--struct-pair)
    - [crates/steamkit/src/steam/handlers/client_msg_handler.rs — 26 lines](#cratessteamkitsrcsteamhandlersclient_msg_handlerrs--26-lines)
    - [crates/steamkit/src/steam/handlers/steam_apps/callbacks.rs — 1476 lines](#cratessteamkitsrcsteamhandlerssteam_appscallbacksrs--1476-lines)
    - [crates/steamkit/src/steam/handlers/steam_apps/steam_apps.rs — 460 lines](#cratessteamkitsrcsteamhandlerssteam_appssteam_appsrs--460-lines)
    - [crates/steamkit/src/steam/handlers/steam_auth_ticket/callbacks.rs — 150 lines](#cratessteamkitsrcsteamhandlerssteam_auth_ticketcallbacksrs--150-lines)
    - [crates/steamkit/src/steam/handlers/steam_auth_ticket/steam_auth_ticket.rs — 423 lines](#cratessteamkitsrcsteamhandlerssteam_auth_ticketsteam_auth_ticketrs--423-lines)
    - [crates/steamkit/src/steam/handlers/steam_auth_ticket/ticket_info.rs — 72 lines](#cratessteamkitsrcsteamhandlerssteam_auth_ticketticket_infors--72-lines)
    - [crates/steamkit/src/steam/handlers/steam_cloud/callbacks.rs — 201 lines](#cratessteamkitsrcsteamhandlerssteam_cloudcallbacksrs--201-lines)
    - [crates/steamkit/src/steam/handlers/steam_cloud/steam_cloud.rs — 157 lines](#cratessteamkitsrcsteamhandlerssteam_cloudsteam_cloudrs--157-lines)
    - [crates/steamkit/src/steam/handlers/steam_content/steam_content.rs — 239 lines](#cratessteamkitsrcsteamhandlerssteam_contentsteam_contentrs--239-lines)
    - [crates/steamkit/src/steam/handlers/steam_friends/callbacks.rs — 1738 lines](#cratessteamkitsrcsteamhandlerssteam_friendscallbacksrs--1738-lines)
    - [crates/steamkit/src/steam/handlers/steam_friends/friend_cache.rs — 176 lines](#cratessteamkitsrcsteamhandlerssteam_friendsfriend_cachers--176-lines)
    - [crates/steamkit/src/steam/handlers/steam_friends/steam_friends.rs — 795 lines](#cratessteamkitsrcsteamhandlerssteam_friendssteam_friendsrs--795-lines)
    - [crates/steamkit/src/steam/handlers/steam_game_coordinator/callbacks.rs — 165 lines](#cratessteamkitsrcsteamhandlerssteam_game_coordinatorcallbacksrs--165-lines)
    - [crates/steamkit/src/steam/handlers/steam_game_coordinator/steam_game_coordinator.rs — 77 lines](#cratessteamkitsrcsteamhandlerssteam_game_coordinatorsteam_game_coordinatorrs--77-lines)
    - [crates/steamkit/src/steam/handlers/steam_game_server/callbacks.rs — 141 lines](#cratessteamkitsrcsteamhandlerssteam_game_servercallbacksrs--141-lines)
    - [crates/steamkit/src/steam/handlers/steam_game_server/steam_game_server.rs — 258 lines](#cratessteamkitsrcsteamhandlerssteam_game_serversteam_game_serverrs--258-lines)
    - [crates/steamkit/src/steam/handlers/steam_master_server/callbacks.rs — 103 lines](#cratessteamkitsrcsteamhandlerssteam_master_servercallbacksrs--103-lines)
    - [crates/steamkit/src/steam/handlers/steam_master_server/steam_master_server.rs — 128 lines](#cratessteamkitsrcsteamhandlerssteam_master_serversteam_master_serverrs--128-lines)
    - [crates/steamkit/src/steam/handlers/steam_matchmaking/callbacks.rs — 516 lines](#cratessteamkitsrcsteamhandlerssteam_matchmakingcallbacksrs--516-lines)
    - [crates/steamkit/src/steam/handlers/steam_matchmaking/lobby_cache.rs — 172 lines](#cratessteamkitsrcsteamhandlerssteam_matchmakinglobby_cachers--172-lines)
    - [crates/steamkit/src/steam/handlers/steam_matchmaking/lobby.rs — 480 lines](#cratessteamkitsrcsteamhandlerssteam_matchmakinglobbyrs--480-lines)
    - [crates/steamkit/src/steam/handlers/steam_matchmaking/steam_matchmaking.rs — 995 lines](#cratessteamkitsrcsteamhandlerssteam_matchmakingsteam_matchmakingrs--995-lines)
    - [crates/steamkit/src/steam/handlers/steam_networking/callbacks.rs — 71 lines](#cratessteamkitsrcsteamhandlerssteam_networkingcallbacksrs--71-lines)
    - [crates/steamkit/src/steam/handlers/steam_networking/steam_networking.rs — 85 lines](#cratessteamkitsrcsteamhandlerssteam_networkingsteam_networkingrs--85-lines)
    - [crates/steamkit/src/steam/handlers/steam_screenshots/callbacks.rs — 64 lines](#cratessteamkitsrcsteamhandlerssteam_screenshotscallbacksrs--64-lines)
    - [crates/steamkit/src/steam/handlers/steam_screenshots/steam_screenshots.rs — 157 lines](#cratessteamkitsrcsteamhandlerssteam_screenshotssteam_screenshotsrs--157-lines)
    - [crates/steamkit/src/steam/handlers/steam_unified_messages/callbacks.rs — 152 lines](#cratessteamkitsrcsteamhandlerssteam_unified_messagescallbacksrs--152-lines)
    - [crates/steamkit/src/steam/handlers/steam_unified_messages/steam_unified_messages.rs — 478 lines](#cratessteamkitsrcsteamhandlerssteam_unified_messagessteam_unified_messagesrs--478-lines)
    - [crates/steamkit/src/steam/handlers/steam_user/callbacks.rs — 852 lines](#cratessteamkitsrcsteamhandlerssteam_usercallbacksrs--852-lines)
    - [crates/steamkit/src/steam/handlers/steam_user/steam_user.rs — 450 lines](#cratessteamkitsrcsteamhandlerssteam_usersteam_userrs--450-lines)
    - [crates/steamkit/src/steam/handlers/steam_user_stats/callbacks.rs — 278 lines](#cratessteamkitsrcsteamhandlerssteam_user_statscallbacksrs--278-lines)
    - [crates/steamkit/src/steam/handlers/steam_user_stats/steam_user_stats.rs — 216 lines](#cratessteamkitsrcsteamhandlerssteam_user_statssteam_user_statsrs--216-lines)
    - [crates/steamkit/src/steam/handlers/steam_workshop/callbacks.rs — 121 lines](#cratessteamkitsrcsteamhandlerssteam_workshopcallbacksrs--121-lines)
    - [crates/steamkit/src/steam/handlers/steam_workshop/steam_workshop.rs — 110 lines](#cratessteamkitsrcsteamhandlerssteam_workshopsteam_workshoprs--110-lines)
  - [5.8 crates/steamkit/src/steam/discovery/ — finding the servers](#58-cratessteamkitsrcsteamdiscovery--finding-the-servers)
  - [5.9 crates/steamkit/src/steam/cdn/ — the depot content CDN](#59-cratessteamkitsrcsteamcdn--the-depot-content-cdn)
  - [5.10 crates/steamkit/src/steam/webapi/ — the HTTP/JSON Web API](#510-cratessteamkitsrcsteamwebapi--the-httpjson-web-api)
  - [5.11 crates/steamkit/src/steam/authentication/ — logging on](#511-cratessteamkitsrcsteamauthentication--logging-on)
    - [5.1 discovery/ — the CM server list](#51-discovery--the-cm-server-list)
    - [crates/steamkit/src/steam/discovery/mod.rs — 9 lines](#cratessteamkitsrcsteamdiscoverymodrs--9-lines)
    - [crates/steamkit/src/steam/discovery/smart_cm_server_list.rs — 899 lines](#cratessteamkitsrcsteamdiscoverysmart_cm_server_listrs--899-lines)
    - [crates/steamkit/src/steam/discovery/server_record.rs — 290 lines](#cratessteamkitsrcsteamdiscoveryserver_recordrs--290-lines)
    - [crates/steamkit/src/steam/discovery/basic_server_list.rs — 78 lines](#cratessteamkitsrcsteamdiscoverybasic_server_listrs--78-lines)
    - [crates/steamkit/src/steam/discovery/basic_server_list_proto.rs — 280 lines](#cratessteamkitsrcsteamdiscoverybasic_server_list_protors--280-lines)
    - [crates/steamkit/src/steam/discovery/i_server_list_provider.rs — 39 lines](#cratessteamkitsrcsteamdiscoveryi_server_list_providerrs--39-lines)
    - [crates/steamkit/src/steam/discovery/file_storage_server_list_provider.rs — 106 lines](#cratessteamkitsrcsteamdiscoveryfile_storage_server_list_providerrs--106-lines)
    - [crates/steamkit/src/steam/discovery/isolated_storage_server_list_provider.rs — 142 lines](#cratessteamkitsrcsteamdiscoveryisolated_storage_server_list_providerrs--142-lines)
    - [crates/steamkit/src/steam/discovery/memory_server_list_provider.rs — 50 lines](#cratessteamkitsrcsteamdiscoverymemory_server_list_providerrs--50-lines)
    - [crates/steamkit/src/steam/discovery/null_server_list_provider.rs — 40 lines](#cratessteamkitsrcsteamdiscoverynull_server_list_providerrs--40-lines)
    - [5.2 cdn/ — content servers, manifests and chunks](#52-cdn--content-servers-manifests-and-chunks)
    - [crates/steamkit/src/steam/cdn/mod.rs — 4 lines](#cratessteamkitsrcsteamcdnmodrs--4-lines)
    - [crates/steamkit/src/steam/cdn/server.rs — 180 lines](#cratessteamkitsrcsteamcdnserverrs--180-lines)
    - [crates/steamkit/src/steam/cdn/depot_chunk.rs — 174 lines](#cratessteamkitsrcsteamcdndepot_chunkrs--174-lines)
    - [crates/steamkit/src/steam/cdn/client.rs — 464 lines](#cratessteamkitsrcsteamcdnclientrs--464-lines)
    - [crates/steamkit/src/steam/cdn/client_lancache.rs — 121 lines](#cratessteamkitsrcsteamcdnclient_lancachers--121-lines)
    - [5.3 webapi/ — the Steam Web API client](#53-webapi--the-steam-web-api-client)
    - [crates/steamkit/src/steam/webapi/mod.rs — 5 lines](#cratessteamkitsrcsteamwebapimodrs--5-lines)
    - [crates/steamkit/src/steam/webapi/web_api.rs — 661 lines](#cratessteamkitsrcsteamwebapiweb_apirs--661-lines)
    - [crates/steamkit/src/steam/webapi/steam_configuration_web_api_extensions.rs — 71 lines](#cratessteamkitsrcsteamwebapisteam_configuration_web_api_extensionsrs--71-lines)
    - [crates/steamkit/src/steam/webapi/web_api_request_exception.rs — 54 lines](#cratessteamkitsrcsteamwebapiweb_api_request_exceptionrs--54-lines)
    - [crates/steamkit/src/steam/webapi/steam_directory.rs — 118 lines](#cratessteamkitsrcsteamwebapisteam_directoryrs--118-lines)
    - [crates/steamkit/src/steam/webapi/content_server_directory_service.rs — 141 lines](#cratessteamkitsrcsteamwebapicontent_server_directory_servicers--141-lines)
    - [5.4 authentication/ — QR and password login](#54-authentication--qr-and-password-login)
    - [The flow, end to end](#the-flow-end-to-end)
    - [crates/steamkit/src/steam/authentication/mod.rs — 10 lines](#cratessteamkitsrcsteamauthenticationmodrs--10-lines)
    - [crates/steamkit/src/steam/authentication/steam_authentication.rs — 509 lines](#cratessteamkitsrcsteamauthenticationsteam_authenticationrs--509-lines)
    - [crates/steamkit/src/steam/authentication/auth_session.rs — 413 lines](#cratessteamkitsrcsteamauthenticationauth_sessionrs--413-lines)
    - [crates/steamkit/src/steam/authentication/auth_session_details.rs — 99 lines](#cratessteamkitsrcsteamauthenticationauth_session_detailsrs--99-lines)
    - [crates/steamkit/src/steam/authentication/qr_auth_session.rs — 124 lines](#cratessteamkitsrcsteamauthenticationqr_auth_sessionrs--124-lines)
    - [crates/steamkit/src/steam/authentication/credentials_auth_session.rs — 90 lines](#cratessteamkitsrcsteamauthenticationcredentials_auth_sessionrs--90-lines)
    - [crates/steamkit/src/steam/authentication/i_authenticator.rs — 29 lines](#cratessteamkitsrcsteamauthenticationi_authenticatorrs--29-lines)
    - [crates/steamkit/src/steam/authentication/user_console_authenticator.rs — 122 lines](#cratessteamkitsrcsteamauthenticationuser_console_authenticatorrs--122-lines)
    - [crates/steamkit/src/steam/authentication/auth_poll_result.rs — 33 lines](#cratessteamkitsrcsteamauthenticationauth_poll_resultrs--33-lines)
    - [crates/steamkit/src/steam/authentication/access_token_generate_result.rs — 24 lines](#cratessteamkitsrcsteamauthenticationaccess_token_generate_resultrs--24-lines)
    - [crates/steamkit/src/steam/authentication/authentication_exception.rs — 56 lines](#cratessteamkitsrcsteamauthenticationauthentication_exceptionrs--56-lines)
- [6. crates/steamkit-tools/ — the dev binaries](#6-cratessteamkit-tools--the-dev-binaries)
  - [6.1 Overview](#61-overview)
    - [Module map](#module-map)
    - [06.2 steamkit-lang-gen — the SteamLanguage (.steamd) pipeline](#062-steamkit-lang-gen--the-steamlanguage-steamd-pipeline)
    - [The .steamd definition language](#the-steamd-definition-language)
    - [The .steamd to Rust pipeline](#the-steamd-to-rust-pipeline)
    - [The alias mechanism (C# duplicate enum values → Rust associated consts)](#the-alias-mechanism-c-duplicate-enum-values--rust-associated-consts)
    - [crates/steamkit-tools/src/lib.rs — 58 lines](#cratessteamkit-toolssrclibrs--58-lines)
    - [crates/steamkit-tools/src/tokenizer.rs — 292 lines](#cratessteamkit-toolssrctokenizerrs--292-lines)
    - [crates/steamkit-tools/src/analyzer.rs — 543 lines](#cratessteamkit-toolssrcanalyzerrs--543-lines)
    - [crates/steamkit-tools/src/generator.rs — 1060 lines](#cratessteamkit-toolssrcgeneratorrs--1060-lines)
    - [crates/steamkit-tools/src/error.rs — 84 lines](#cratessteamkit-toolssrcerrorrs--84-lines)
    - [crates/steamkit-tools/src/main.rs — 165 lines](#cratessteamkit-toolssrcmainrs--165-lines)
    - [06.3 steamkit-proto-gen — regenerating the prost code](#063-steamkit-proto-gen--regenerating-the-prost-code)
    - [The protobuf to Rust pipeline](#the-protobuf-to-rust-pipeline)
    - [crates/steamkit-tools/src/proto_gen/mod.rs — 7 lines](#cratessteamkit-toolssrcproto_genmodrs--7-lines)
    - [crates/steamkit-tools/src/proto_gen/proto_set.rs — 219 lines](#cratessteamkit-toolssrcproto_genproto_setrs--219-lines)
    - [crates/steamkit-tools/src/proto_gen/code_generator.rs — 244 lines](#cratessteamkit-toolssrcproto_gencode_generatorrs--244-lines)
    - [crates/steamkit-tools/src/bin/steamkit-proto-gen.rs — 225 lines](#cratessteamkit-toolssrcbinsteamkit-proto-genrs--225-lines)
    - [crates/steamkit-tools/src/proto_error.rs — 94 lines](#cratessteamkit-toolssrcproto_errorrs--94-lines)
    - [06.4 steamkit-proto-dump](#064-steamkit-proto-dump)
    - [crates/steamkit-tools/src/proto_dumper/mod.rs — 19 lines](#cratessteamkit-toolssrcproto_dumpermodrs--19-lines)
    - [crates/steamkit-tools/src/proto_dumper/wire.rs — 244 lines](#cratessteamkit-toolssrcproto_dumperwirers--244-lines)
    - [crates/steamkit-tools/src/proto_dumper/raw_options.rs — 441 lines](#cratessteamkit-toolssrcproto_dumperraw_optionsrs--441-lines)
    - [crates/steamkit-tools/src/proto_dumper/executable_scanner.rs — 120 lines](#cratessteamkit-toolssrcproto_dumperexecutable_scannerrs--120-lines)
    - [crates/steamkit-tools/src/proto_dumper/protobuf_collector.rs — 65 lines](#cratessteamkit-toolssrcproto_dumperprotobuf_collectorrs--65-lines)
    - [crates/steamkit-tools/src/proto_dumper/protobuf_dumper.rs — 1918 lines](#cratessteamkit-toolssrcproto_dumperprotobuf_dumperrs--1918-lines)
    - [crates/steamkit-tools/src/proto_dumper/util.rs — 119 lines](#cratessteamkit-toolssrcproto_dumperutilrs--119-lines)
    - [crates/steamkit-tools/src/bin/steamkit-proto-dump.rs — 374 lines](#cratessteamkit-toolssrcbinsteamkit-proto-dumprs--374-lines)
    - [06.5 steamkit-nethook-analyzer](#065-steamkit-nethook-analyzer)
    - [The on-disk contract with the NetHook2 DLL](#the-on-disk-contract-with-the-nethook2-dll)
    - [The message reader and the protobuf-header subtlety](#the-message-reader-and-the-protobuf-header-subtlety)
    - [crates/steamkit-tools/src/nethook/mod.rs — 36 lines](#cratessteamkit-toolssrcnethookmodrs--36-lines)
    - [crates/steamkit-tools/src/nethook/error.rs — 81 lines](#cratessteamkit-toolssrcnethookerrorrs--81-lines)
    - [crates/steamkit-tools/src/nethook/item.rs — 250 lines](#cratessteamkit-toolssrcnethookitemrs--250-lines)
    - [crates/steamkit-tools/src/nethook/dump.rs — 134 lines](#cratessteamkit-toolssrcnethookdumprs--134-lines)
    - [crates/steamkit-tools/src/nethook/message_reading.rs — 304 lines](#cratessteamkit-toolssrcnethookmessage_readingrs--304-lines)
    - [crates/steamkit-tools/src/nethook/message_type_finder.rs — 116 lines](#cratessteamkit-toolssrcnethookmessage_type_finderrs--116-lines)
    - [crates/steamkit-tools/src/nethook/message_type_overrides.rs — 194 lines](#cratessteamkit-toolssrcnethookmessage_type_overridesrs--194-lines)
    - [crates/steamkit-tools/src/nethook/proto_registry.rs — 10 621 lines](#cratessteamkit-toolssrcnethookproto_registryrs--10-621-lines)
    - [crates/steamkit-tools/src/nethook/serializable_registry.rs — 77 lines](#cratessteamkit-toolssrcnethookserializable_registryrs--77-lines)
    - [crates/steamkit-tools/src/nethook/emsg_extensions.rs — 92 lines](#cratessteamkit-toolssrcnethookemsg_extensionsrs--92-lines)
    - [crates/steamkit-tools/src/nethook/well_known_app_ids.rs — 13 lines](#cratessteamkit-toolssrcnethookwell_known_app_idsrs--13-lines)
    - [crates/steamkit-tools/src/nethook/protobuf_field_reader.rs — 155 lines](#cratessteamkit-toolssrcnethookprotobuf_field_readerrs--155-lines)
    - [crates/steamkit-tools/src/nethook/body.rs — 154 lines](#cratessteamkit-toolssrcnethookbodyrs--154-lines)
    - [crates/steamkit-tools/src/nethook/render.rs — 304 lines](#cratessteamkit-toolssrcnethookrenderrs--304-lines)
    - [crates/steamkit-tools/src/nethook/tree.rs — 111 lines](#cratessteamkit-toolssrcnethooktreers--111-lines)
    - [crates/steamkit-tools/src/nethook/specializations/mod.rs — 84 lines](#cratessteamkit-toolssrcnethookspecializationsmodrs--84-lines)
    - [crates/steamkit-tools/src/nethook/specializations/client_service_method.rs — 85 lines](#cratessteamkit-toolssrcnethookspecializationsclient_service_methodrs--85-lines)
    - [crates/steamkit-tools/src/nethook/specializations/gc_generic.rs — 106 lines](#cratessteamkit-toolssrcnethookspecializationsgc_genericrs--106-lines)
    - [crates/steamkit-tools/src/nethook/specializations/mms_lobby.rs — 118 lines](#cratessteamkit-toolssrcnethookspecializationsmms_lobbyrs--118-lines)
    - [crates/steamkit-tools/src/nethook/specializations/multi_packet.rs — 86 lines](#cratessteamkit-toolssrcnethookspecializationsmulti_packetrs--86-lines)
    - [crates/steamkit-tools/src/nethook/specializations/remote_client_steam.rs — 231 lines](#cratessteamkit-toolssrcnethookspecializationsremote_client_steamrs--231-lines)
    - [crates/steamkit-tools/src/nethook/specializations/shared_objects.rs — 384 lines](#cratessteamkit-toolssrcnethookspecializationsshared_objectsrs--384-lines)
    - [crates/steamkit-tools/src/bin/steamkit-nethook-analyzer.rs — 346 lines](#cratessteamkit-toolssrcbinsteamkit-nethook-analyzerrs--346-lines)
    - [06.6 crates/steamkit-tools/lang/ — the Valve definition files](#066-cratessteamkit-toolslang--the-valve-definition-files)
    - [crates/steamkit-tools/lang/*.steamd — 7 files, 4 404 lines total](#cratessteamkit-toolslangsteamd--7-files-4-404-lines-total)
    - [06.7 Cargo.toml and build.rs](#067-cargotoml-and-buildrs)
    - [crates/steamkit-tools/Cargo.toml — 53 lines](#cratessteamkit-toolscargotoml--53-lines)
    - [crates/steamkit-tools/build.rs — none](#cratessteamkit-toolsbuildrs--none)
    - [06.8 The in-crate verification script](#068-the-in-crate-verification-script)
    - [crates/steamkit-tools/verify_steam_language.py — 284 lines](#cratessteamkit-toolsverify_steam_languagepy--284-lines)
    - [crates/steamkit-tools/tools/gen_proto_registry.py — 485 lines](#cratessteamkit-toolstoolsgen_proto_registrypy--485-lines)
    - [crates/steamkit-tools/src/bin/steamkit-depot-dump.rs - 125 lines](#cratessteamkit-toolssrcbinsteamkit-depot-dumprs---125-lines)
- [7. steamkit-samples and steamkit-tests](#7-steamkit-samples-and-steamkit-tests)
  - [7.1 crates/steamkit-samples - the ported sample programs](#71-cratessteamkit-samples---the-ported-sample-programs)
    - [7.1.1 Layout](#711-layout)
    - [7.1.2 Sample index](#712-sample-index)
    - [7.1.3 How the ports differ from the C# projects](#713-how-the-ports-differ-from-the-c-projects)
    - [7.1.4 mod qr_code - the shared QR renderer](#714-mod-qr_code---the-shared-qr-renderer)
    - [7.1.5 The binaries](#715-the-binaries)
  - [7.2 crates/steamkit-tests - the ported xunit suite](#72-cratessteamkit-tests---the-ported-xunit-suite)
    - [7.2.1 Layout](#721-layout)
    - [7.2.2 tests/common/mod.rs - the shared fixtures](#722-testscommonmodrs---the-shared-fixtures)
    - [7.2.3 tests/data/ - the binary fixtures](#723-testsdata---the-binary-fixtures)
    - [7.2.4 The ignored tests](#724-the-ignored-tests)
    - [7.2.5 Current result](#725-current-result)
    - [7.2.6 The test files](#726-the-test-files)
    - [7.2.7 The test-suite audit](#727-the-test-suite-audit)
    - [7.2.8 The Rust-only tests of the networking layer and the CM client lifecycle](#728-the-rust-only-tests-of-the-networking-layer-and-the-cm-client-lifecycle)
  - [7.3 What the suite found](#73-what-the-suite-found)
  - [7.4 Running the suite](#74-running-the-suite)
    - [The live depot test](#the-live-depot-test)
- [8. crates/steamkit-nethook/ — the NetHook2 DLL](#8-cratessteamkit-nethook--the-nethook2-dll)
  - [8.1 Overview](#81-overview)
    - [8.1 What NetHook2 is for](#81-what-nethook2-is-for)
    - [8.2 The C++ original and what the port changed](#82-the-c-original-and-what-the-port-changed)
    - [8.3 The crate is Windows-only and is the only unsafe in the workspace](#83-the-crate-is-windows-only-and-is-the-only-unsafe-in-the-workspace)
    - [8.4 The on-disk contract with the analyzer](#84-the-on-disk-contract-with-the-analyzer)
    - [8.5 What has and has not been verified](#85-what-has-and-has-not-been-verified)
    - [8.6 File map](#86-file-map)
    - [crates/steamkit-nethook/Cargo.toml — 43 lines](#cratessteamkit-nethookcargotoml--43-lines)
    - [crates/steamkit-nethook/build.rs — 39 lines](#cratessteamkit-nethookbuildrs--39-lines)
    - [crates/steamkit-nethook/src/lib.rs — 34 lines](#cratessteamkit-nethooksrclibrs--34-lines)
    - [crates/steamkit-nethook/src/nethook.rs — 109 lines](#cratessteamkit-nethooksrcnethookrs--109-lines)
    - [crates/steamkit-nethook/src/injector.rs — 1103 lines](#cratessteamkit-nethooksrcinjectorrs--1103-lines)
    - [crates/steamkit-nethook/src/logger.rs — 366 lines](#cratessteamkit-nethooksrcloggerrs--366-lines)
    - [crates/steamkit-nethook/src/net.rs — 232 lines](#cratessteamkit-nethooksrcnetrs--232-lines)
    - [crates/steamkit-nethook/src/crypto.rs — 234 lines](#cratessteamkit-nethooksrccryptors--234-lines)
    - [crates/steamkit-nethook/src/csimpledetour.rs — 909 lines](#cratessteamkit-nethooksrccsimpledetourrs--909-lines)
    - [crates/steamkit-nethook/src/csimplescan.rs — 92 lines](#cratessteamkit-nethooksrccsimplescanrs--92-lines)
    - [crates/steamkit-nethook/src/sigscan.rs — 198 lines](#cratessteamkit-nethooksrcsigscanrs--198-lines)
    - [crates/steamkit-nethook/src/sedebug.rs — 120 lines](#cratessteamkit-nethooksrcsedebugrs--120-lines)
    - [crates/steamkit-nethook/src/binaryreader.rs — 69 lines](#cratessteamkit-nethooksrcbinaryreaderrs--69-lines)
    - [crates/steamkit-nethook/src/nh2_string.rs — 29 lines](#cratessteamkit-nethooksrcnh2_stringrs--29-lines)
    - [crates/steamkit-nethook/src/steamclient.rs — 8 lines](#cratessteamkit-nethooksrcsteamclientrs--8-lines)
    - [crates/steamkit-nethook/src/version.rs — 25 lines](#cratessteamkit-nethooksrcversionrs--25-lines)
    - [crates/steamkit-nethook/src/zip.rs — 33 lines](#cratessteamkit-nethooksrcziprs--33-lines)
    - [crates/steamkit-nethook/src/steam/mod.rs — 7 lines](#cratessteamkit-nethooksrcsteammodrs--7-lines)
    - [crates/steamkit-nethook/src/steam/emsg.rs — 38 lines](#cratessteamkit-nethooksrcsteamemsgrs--38-lines)
    - [crates/steamkit-nethook/src/steam/net.rs — 77 lines](#cratessteamkit-nethooksrcsteamnetrs--77-lines)
    - [crates/steamkit-nethook/src/steam/clientmsgs.rs — 110 lines](#cratessteamkit-nethooksrcsteamclientmsgsrs--110-lines)
    - [crates/steamkit-nethook/src/steam/csteamid.rs — 279 lines](#cratessteamkit-nethooksrcsteamcsteamidrs--279-lines)
    - [crates/steamkit-nethook/src/steam/steamtypes.rs — 361 lines](#cratessteamkit-nethooksrcsteamsteamtypesrs--361-lines)
    - [crates/steamkit-nethook/src/steam/udppkt.rs — 126 lines](#cratessteamkit-nethooksrcsteamudppktrs--126-lines)
    - [8.7 The unsafe discipline, file by file](#87-the-unsafe-discipline-file-by-file)
    - [8.8 End-to-end data flow](#88-end-to-end-data-flow)
    - [8.9 Summary of deviations from the C++ original](#89-summary-of-deviations-from-the-c-original)
- [9. The generated and vendored code](#9-the-generated-and-vendored-code)
  - [9.1 vendor/Protobufs/ — Valve's protobuf definitions](#91-vendorprotobufs--valves-protobuf-definitions)
  - [9.2 crates/steamkit-protos/proto/ — the staged subset](#92-cratessteamkit-protosproto--the-staged-subset)
  - [9.3 crates/steamkit/src/base/generated/steam_language.rs — the 91 Valve enums](#93-cratessteamkitsrcbasegeneratedsteam_languagers--the-91-valve-enums)
  - [9.4 The protobuf code itself is not on disk](#94-the-protobuf-code-itself-is-not-on-disk)
  - [9.5 crates/steamkit-tools/lang/*.steamd](#95-cratessteamkit-toolslangsteamd)
  - [9.6 What is *not* generated](#96-what-is-not-generated)
- [10. Known deviations from the C#](#10-known-deviations-from-the-c)
  - [10.1 The CMWebSocket HTTP client purpose does not reach the transport](#101-the-cmwebsocket-http-client-purpose-does-not-reach-the-transport)
  - [10.2 The websocket URI needs the host name](#102-the-websocket-uri-needs-the-host-name)
  - [10.3 Event ordering](#103-event-ordering)
  - [10.4 event → broadcast for the *outbound* path](#104-event--broadcast-for-the-outbound-path)
  - [10.5 Unknown protobuf fields are dropped](#105-unknown-protobuf-fields-are-dropped)
  - [10.6 The samples print non-ASCII as ?](#106-the-samples-print-non-ascii-as-)
  - [10.7 macOS and Windows-only machine info](#107-macos-and-windows-only-machine-info)
  - [10.8 DNS is resolved eagerly](#108-dns-is-resolved-eagerly)
  - [10.9 The transport order is TCP, websocket, UDP — not websocket, TCP, UDP](#109-the-transport-order-is-tcp-websocket-udp--not-websocket-tcp-udp)
- [11. Building, running, verifying](#11-building-running-verifying)
  - [11.1 The RAM and disk policy — read this first](#111-the-ram-and-disk-policy--read-this-first)
  - [11.2 Build](#112-build)
  - [11.3 The four verification gates](#113-the-four-verification-gates)
  - [11.4 The test suite](#114-the-test-suite)
  - [11.5 Running the samples](#115-running-the-samples)
  - [11.6 The dev tools](#116-the-dev-tools)
- [12. The bug history](#12-the-bug-history)
- [13. The original C# repository](#13-the-original-c-repository)
- [14. Where to go next](#14-where-to-go-next)
  - [14.1 What has and has not been exercised](#141-what-has-and-has-not-been-exercised)
- [15. What has been verified against live Steam](#15-what-has-been-verified-against-live-steam)
  - [15.1 Verified](#151-verified)
  - [15.2 What still needs a person](#152-what-still-needs-a-person)
  - [15.3 What is still unproven](#153-what-is-still-unproven)

---

## 1. What this is

SteamKit2 is a C# client library for Valve's Steam protocol. It speaks three protocols at
once:

- **Steam3 / CM** — the binary protocol the `Steam.exe` client uses. Framed, encrypted,
  multiplexed over TCP, UDP or a websocket.
- **Protobuf** — most CM messages are protobuf-encoded. The message definitions are Valve's
  own and are not public API in any useful sense.
- **The Web API** — the HTTP/JSON API behind `store.steampowered.com` and friends, used for
  store data, news, and account lookups.

This port converts **the entire repository** — the library, the 14 samples, the test suite,
and the `Resources/` tooling including the C++ `NetHook2` DLL — to Rust.

The port is **behaviour-preserving**, not a redesign. Where Rust forces a different shape,
the change is recorded in the porting notes with the reason. Where the C# is
ambiguous, the C# wins.

### 1.1 The single most important thing to know

**A port that compiles is not a port that works.**

The test suite was ported but never compiled until the very end of the project, because the
workers had been told not to run `cargo`. The moment it was compiled and run, it exposed
**eight real bugs** in code that had compiled cleanly for weeks. Running the ported *tools*
and *samples* exposed two more.

Twelve defects, all of them in the parts everyone assumed were safe: AES key ordering, a
buffer that was never written, an integer underflow the C# tolerates and Rust panics on, a
byte order mark nobody detected, a decompression that wrote its output at the wrong offset,
and a network transport that existed but was never wired up.

**Run the tests before you believe anything.** See [§11](#11-building-running-verifying).

### 1.2 The standing constraints

These still apply to anyone working on the port:

- **The C# tree is READ-ONLY.** Never write to `csharp-original/`, nor to the upstream
  checkout it was copied from. All output goes to `crates/`.
- **Subagents:** follow the user's current tier policy — every allowed subagent runs on
  `opencode-go/deepseek-v4.1-flash`; `build` is prohibited; `oracle` needs a one-time yes.
- **No git commits** unless the user asks.
- **Report CLOCK time**, never a bare duration.

### 1.3 The documents beside this one

| Document | What it is | Read it when |
|---|---|---|
| `SUMMARY.md` | the entry point: status, layout, architecture, key flows, conventions and deviations | you are new to the tree or resuming the project |
| `DEPENDENCIES.md` | dependency inventory, upgrade chain, and the SteamKit2/protobuf refresh checklist | you are bumping a pin or reviewing maintenance |
| `FILE_INDEX.md` | every module, with the line of this guide that documents it | you want the guide section for a specific file |

---

## 2. Repository layout

```
steamkit-rs/
  README.md
  Cargo.toml
  Cargo.lock
  crates/
    SUMMARY.md              the entry point and condensed guide
    DEPENDENCIES.md         dependency inventory and upgrade checklist
    DEVELOPER_GUIDE.md      this file
    FILE_INDEX.md           every module, with the line of this guide that documents it
    steamkit/               THE LIBRARY
      src/
        base/               message framing, the wire format layer
        error.rs            the single error type
        networking/         the Steam3 transports
        steam/              the client, the handlers, the subsystems
        types/              the value types (SteamID, KeyValue, JobID...)
        util/               the helpers, incl. the vendored LZMA SDK
    steamkit-protos/        protobuf types, generated by prost at build time
    steamkit-tools/         the dev binaries (the C# Resources tooling)
    steamkit-samples/       14 runnable samples
    steamkit-tests/         the ported xunit suite
    steamkit-nethook/       the Windows DLL (the C++ NetHook2)
```

### 2.1 The crate dependency graph

```
                    steamkit-protos
                    (generated)   │
              ┌──────────┬────────┼────────┬──────────┐
              │          │        │        │          │
           steamkit   tools   samples   tests    nethook
              │          │        │        │          │
              └──────────┴────────┴────────┘
```

| Crate | Depends on | Purpose |
|---|---|---|
| `steamkit` | `steamkit-protos` | the library. Everything else depends on it |
| `steamkit-protos` | — | 6,657 protobuf structs + 852 enums, generated by `prost` at build time |
| `steamkit-tools` | `steamkit`, `steamkit-protos` | the four dev binaries |
| `steamkit-samples` | `steamkit`, `steamkit-protos` | 14 runnable programs |
| `steamkit-tests` | `steamkit`, `steamkit-tools`, `steamkit-protos` | the ported xunit suite |
| `steamkit-nethook` | `steamkit-protos` **only** | the Windows DLL. Deliberately standalone: it must not drag the whole library into an injected process |

`steamkit-nethook` depending on nothing but the generated protobuf types is a deliberate
constraint. A DLL injected into `Steam.exe` should carry as little as possible.

---

## 3. The architecture

### 3.1 The shape of the library

`crates/steamkit/src/` has six public modules:

| Module | Contains | C# counterpart |
|---|---|---|
| `base` | message framing: `IClientMsg`, `IPacketMsg`, `MsgUtil`, the wire-format structs | `SteamKit2/Base/` |
| `error` | the single `SteamError` type | — (the C# throws) |
| `networking` | the Steam3 transports: TCP, UDP, websocket, the envelope encryption | `SteamKit2/Networking/Steam3/` |
| `steam` | `SteamClient`, `CMClient`, the 15 handlers, discovery, CDN, Web API, authentication | `SteamKit2/Steam/` |
| `types` | the value types: `SteamID`, `JobID`, `KeyValue`, depot manifests | `SteamKit2/Types/` |
| `util` | helpers: crypto, LZMA, zip, machine info, logging | `SteamKit2/Util/` |

### 3.2 How a message flows

This is the path every message takes, from the socket to your callback.

```
   ┌──────────────┐
   │  TcpConnection / UdpConnection / WebSocketConnection   (networking/steam3)
   │      implements IConnection: connect, disconnect, send, net_msg_received
   └──────────────┘
            │  raw bytes
            ▼
   ┌──────────────┐
   │ EnvelopeEncryptedConnection        (networking/steam3)
   │  reads the 4-byte message number, strips the 0x80000000 protobuf flag,
   │  handles the channel-encrypt handshake, decrypts, and re-frames
   └──────────────┘
            │  decrypted, framed
            ▼
   ┌──────────────┐
   │ CMClient                                     (steam/cm_client.rs)
   │  parses the header (MsgHdrProtoBuf / MsgHdr / ExtendedClientMsgHdr),
   │  decides proto vs non-proto, builds a ClientMsgProtobuf
   │  ── the dispatch loop owns a ConnectionSession: the connection and
   │     its ordered event stream, so the whole session can be replaced
   └──────────────┘
            │  IClientMsg
            ▼
   ┌──────────────┐
   │ SteamClient → the handler registry           (steam/steam_client/steam_client.rs)
   │  looks up the handler that owns this EMsg
   └──────────────┘
            │
            ▼
   ┌──────────────┐
   │ <Handler>                                     (steam/handlers/<name>/)
   │  implements ClientMsgHandler, decodes the body, and emits a callback
   └──────────────┘
            │  callback struct
            ▼
   ┌──────────────┐
   │ CallbackMgr                                   (steam/steam_client/callback_mgr/)
   │  dispatches to the subscribers of that callback type
   └──────────────┘
            │
            ▼
   your `client.subscribe::<ConnectedCallback>(...)`
```

Two details in that diagram catch people out:

**The message number is read twice, in two different places, for a reason.** For a protobuf
message the *header* carries the message number itself (`MsgHdrProtoBuf` serialises the EMsg
as its first field), whereas the older binary headers (`MsgHdr`, `ExtendedClientMsgHdr`)
*follow* the message number. The C# has the same asymmetry; a reader of the port has to
preserve it. Getting it wrong makes the analyzer misread the body as a header length.

**`MsgUtil::is_proto_buf` tests bit 31 of the message number**, not a numeric range. A
message number with `0x80000000` set is protobuf; anything else uses the binary header.

### 3.2.1 There is no server-switch path

The C# marks a server bad and leaves the reconnection to the caller; its samples give up when the
resulting `LoggedOnCallback` reports `TryAnotherCM`. The Rust port does the same: a connection that
ends ends the session, and the caller calls `connect` again. An earlier revision moved to another CM
server by itself and replayed the logon; that was removed for parity (the porting notes §20.1).

### 3.3 The async model

The C# is `async`/`await` throughout. The port is tokio, and the mapping is mechanical:

| C# | Rust |
|---|---|
| `Task` / `Task<T>` | `async fn` / `Future<Output = T>` |
| `TaskCompletionSource<T>` | `tokio::sync::oneshot::Sender` / `Receiver` |
| `CancellationToken` | `tokio_util::sync::CancellationToken` (same crate, same name) |
| `event` / `EventHandler<T>` | `tokio::sync::broadcast::Sender<T>` / `Receiver<T>`; the connection events also as one ordered `mpsc` stream |
| `lock` / `SemaphoreSlim` | `parking_lot` locks / `tokio::sync::Semaphore` |
| `IProgress<T>` | a `broadcast` channel |
| `Timer` / `Task.Delay` | `tokio::time::sleep` / `tokio::time::interval` |

**The one real semantic difference: event ordering.** A C# `event` invokes its handlers *in
registration order*, synchronously, one after the other, and every handler sees every event.
A tokio `broadcast` channel gives every subscriber the same message but delivers it to each of
them independently, and a slow subscriber can lag and lose messages. That is why the
connection events - which drive the protocol - are also offered as one ordered, unbounded
stream per subscriber (`ConnectionEvents::events`), and why everything in the library uses it
(the porting notes §14). Where the C# relied on handler order, the port documents it.

### 3.4 The error model

The C# throws. The port does not.

There is exactly one error type in the library, `SteamError`, defined in
`crates/steamkit/src/error.rs`, with a `thiserror` enum of variants (`Malformed`,
`Connection`, `Timeout`, `InvalidArgument`, `Decompression`, `Crypto`, …). Every fallible
function returns `Result<T, SteamError>`.

The rules:
- **No `unwrap()` or `expect()` on runtime-supplied data** in library code. A malformed
  message from the network is a `SteamError`, never a panic.
- The `SteamError` variants are *not* a one-to-one map of the C# exception types. The C#
  `SteamKitWebRequestException` and friends become variants; the point is that the caller
  can match on the *cause*, not on the exception hierarchy.
- `unsafe` is forbidden crate-wide, so there is no panic-on-invalid-UTF8 escape hatch
  either.

### 3.5 The protobuf strategy

The C# hand-writes **265,000 lines** of protobuf classes in `SteamKit2/Base/Generated/`.
The port does not hand-write them and does not check them in either — it **generates** them
from Valve's canonical definitions:

```
vendor/Protobufs/            Valve's protos, pinned at commit bdb3957d
        │
        │  tools/stage_protos.py
        ▼
crates/steamkit-protos/proto/   155 staged .proto files, 7 proto sets
        │
        │  prost-build (in build.rs)
        ▼
$OUT_DIR/steam/steam.rs      6,657 structs + 852 enums
        │
        │  include!(concat!(env!("OUT_DIR"), "/steam/steam.rs"))
        ▼
crates/steamkit-protos/src/lib.rs
```

`stage_protos.py` applies exactly two patches, neither of which changes the wire format:

| Patch | Why |
|---|---|
| strips Valve's `option additional_includes` | not standard protobuf; protoc 36 rejects it |
| strips the `boxed_type` and `synthetic_default` field options | unknown to protoc 36; they are code-generation hints only |

The seven proto sets map onto the C# namespaces one-to-one:

| Rust module | C# namespace |
|---|---|
| `steam` | `SteamKit2.Internal` |
| `webui` | `SteamKit2.WebUI.Internal` |
| `csgo` | `SteamKit2.GC.CSGO.Internal` |
| `dota2` | `SteamKit2.GC.Dota.Internal` |
| `tf2` | `SteamKit2.GC.TF2.Internal` |
| `deadlock` | `SteamKit2.GC.Deadlock.Internal` |
| `gc` | `SteamKit2.GC.Internal` |

### 3.6 The 91 Valve enums

The enums Valve keeps in `.steamd` definition files rather than in protobuf — `EMsg`,
`EResult`, `EUniverse`, `EAccountType`, and 87 more — are generated by `steamkit-lang-gen`,
a Rust port of the C# `Resources/SteamLanguageParser`. The output is
`crates/steamkit/src/base/generated/steam_language.rs` (17,863 lines).

**The alias rule.** The C# allows duplicate enum values; Rust does not. The C# has, for
example, `EMsg.BaseGeneral = 100` and `EMsg.GenericReply = 100` as two separate members.
The generator emits the first as a real variant and every later one as an associated
`const` on it:

```rust
/// Alias of [`EMsg::BaseGeneral`].
pub const GenericReply: EMsg = EMsg::BaseGeneral;
```

3,054 real variants plus 39 aliases gives the C#'s 3,093 members. `tools/verify_steam_language.py`
checks all 91 enums and every variant against the C#.

### 3.7 The wire-format layer

`crates/steamkit/src/base/generated/steam_language_internal.rs` holds the hand-written half
of Steam's wire format: `UdpHeader`, `MsgHdr`, `ExtendedClientMsgHdr`, the chat/ban/clan
structs. Getting one field width wrong silently corrupts every message, so this layer was
built by hand and is checked by a script.

The macro that drives it deserves a note, because the obvious approach does not work:

```rust
// THIS DOES NOT COMPILE, and no spelling of it does:
macro_rules! write_field {
    ($kind:ty, $v:expr) => { match $v { u32 => ..., u64 => ... } };  // $kind is opaque
}
```

A `macro_rules!` fragment captured as `ty` is **opaque**: it cannot be re-matched against a
literal token. And `en(EMsg)` is *two* token trees, so a `tt` fragment cannot capture it
either. There is no fragment spelling that dispatches on the field's type.

The resolution is a trait — the field's own type carries the behaviour:

```rust
pub trait WireValue {
    fn write_wire(&self, out: &mut Vec<u8>);
    fn read_wire(r: &mut Reader<'_>) -> Result<Self, SteamError>;
}
```

`EUdpPacketType` is **one byte** on the wire (the C# writes `(byte)PacketType`) while every
other Valve enum is four. The trait's `wire_size()` carries that.

`tools/verify_wire_format.py` parses the C# `bw.Write(...)` calls per struct and compares
the widths against the Rust field kinds: 145 fields, 0 mismatches.

---

## 4. The C# → Rust mapping

### 4.1 Names that changed

| C# | Rust | Why |
|---|---|---|
| `AsyncJob` / `AsyncJob<T>` | `JobState` / `JobHandle<T>` / `JobGroup<T>` | Rust cannot have two types with the same name in one module. The C#'s `AsyncJob` is a class with a generic sibling; the port splits them into a state type and a handle type |
| `KeyValue` (a class with `Name`/`Value`/`Children`) | `KeyValue` (a struct with the same fields) | a direct port; the nested `Type` enum became a private field |
| `EventArgs` classes | plain structs | the C# ones are classes with an `EventHandler` pattern; Rust uses `broadcast` channels |
| `Exception` types | `SteamError` variants | one error type instead of a hierarchy |
| `IDisposable` | `Drop` | automatic |
| `byte[]` | `Vec<u8>` / `&[u8]` / `Bytes` | `Bytes` where the data is shared (the network path) |

### 4.2 Names that did **not** change

The frozen public API of `SteamClient` is byte-for-byte the C#'s, including the method
names. the porting contract §14.3 freezes it. Same for `JobID`, `SteamID`, `IClientMsg`,
`IPacketMsg`, `CallbackMsg`, `ClientMsgHandler`, `HttpClientPurpose`, `ProtocolTypes`, and
every `E*` enum.

### 4.3 Idioms that changed shape

| C# | Rust | Note |
|---|---|---|
| `class XHandler : ClientMsgHandler` | `trait XHandler` + `struct X` | Rust has no inheritance |
| `event EventHandler<T>` | `broadcast::Sender<T>` | ordering caveat, see [§3.3](#33-the-async-model) |
| `CancellationTokenSource` | `CancellationToken` | the token *is* the source in tokio-util |
| `async void` | `async fn` returning `()` | — |
| `IEnumerable<T>` with `yield` | `impl Stream` or a `Vec` | the port uses `Vec` unless laziness matters |
| `IDisposable` on the connection | `Drop` + an explicit `disconnect` | the port's `disconnect` is async |
| `static void` helpers | free functions or associated functions | — |
| `Nullable<T>` | `Option<T>` | — |
| `out` / `ref` parameters | `&mut` | — |
| `Tuple` | tuple | — |

### 4.4 The full C# → Rust substitution table

The complete list of C# constructs that needed a decision, and what the port settled on.
Use this when you are porting a snippet and need to know what to reach for.

| C# | Rust |
|---|---|
| `event EventHandler<T>` | `tokio::sync::broadcast::Sender<T>`; subscribers get a `Receiver` |
| `Action<T>` | `Box<dyn Fn(T) + Send + Sync>` |
| `Func<T, Task>` | `Box<dyn Fn(T) -> BoxFuture<'static, ()> + Send + Sync>` |
| `IDisposable` | `Drop` |
| `lock` | `parking_lot::Mutex` (sync) / `tokio::sync::Mutex` (held across await) |
| `ConcurrentDictionary` | `dashmap::DashMap` |
| `System.Threading.Timer` | `tokio::time::interval` in a spawned task |
| `Thread` + blocking socket | `tokio::spawn` + `tokio::net::TcpStream` |
| `Socket.Poll` | `stream.readable().await` + `stream.try_read()` |
| `ClientWebSocket` | `tokio-tungstenite` |
| `HttpClient` / `HttpWebRequest` | `reqwest` |
| `System.IO.Hashing.Crc32` | `crc32fast` |
| `ZstdSharp.Port` | `zstd` |
| `GZipStream` | `flate2` |
| `Aes` / `RSA` / `HMACSHA1` | `aes`+`cbc`+`ecb` / `rsa` / `hmac`+`sha1` |
| `Process.GetCurrentProcess().StartTime` | process start time captured once, in a `OnceLock` |
| `IsolatedStorage` | a directory under `std::env::temp_dir()` (a recorded deviation) |
| `System.IO.Hashing.XxHash` | `twox-hash` |
| `System.Web.HttpUtility` | `percent-encoding` + `urlencoding` |
| `System.Dynamic.DynamicObject` | `serde_json::Value` |
| `System.Text.RegularExpressions` | `regex` |
| `ImmutableList` + `ImmutableInterlocked` | clone-on-write `RwLock<Vec<..>>` |

### 4.5 The design decisions that were hard, and what they resolved to

These are the places where the C# and Rust genuinely did not fit together, and what was
decided. Knowing them saves re-deriving the same answers.

| Problem | Resolution |
|---|---|
| The C# has `AsyncJob` **and** `AsyncJob<T>`; Rust cannot have two types with the same name in one module | Resolved as **one generic type with a default parameter** — `AsyncJob<T = AnyCallback>` in `types::job_id` — so `AsyncJob` (untyped) and `AsyncJob<T>` (typed) both exist. The runtime job registry is `steam_client::async_job_manager` with `JobHandle<T>` |
| The wire macro needed to dispatch on the field's type, but a `macro_rules!` fragment captured as `ty` is opaque and `en(EMsg)` is two token trees — no fragment spelling can re-match either | The macro dispatches on the field's **Rust type** via a `WireValue` trait, with a catch-all arm for `WireEnum` types. 36 call sites were updated |
| The contract originally described `KeyValue` as an enum | The C# `KeyValue` is a **class** (`Types/KeyValue.cs:220`) with `Name`/`Value`/`Children` and a private nested `Type` enum. The contract was corrected; the Rust type matches the C# |
| `chrono` and `percent-encoding` were used but not declared | Both added to `Cargo.toml` |
| `IMachineInfoProvider` trait methods did not match their implementations | Renamed to match the implementations |
| `CancellationToken::none()` does not exist | `CancellationToken::new()` |
| `ClientMsgProtobuf::new` had lost the C#'s `flags` argument | Restored |
| `KeyValueSet` did not exist — an artefact of the wrong `KeyValue` spec | Removed |

### 4.6 The two seams the port invented

The C# has no equivalent, because the C# has no need for them:

**`ConnectionFactory`** — the C#'s `CMClient.CreateConnection` is a private method that
picks a transport. The port makes it a seam, because the transport choice is environment
specific:

```rust
pub type ConnectionFactory = Arc<
    dyn Fn(&CMClient, &ServerRecord, ProtocolTypes) -> Result<Arc<dyn Connection>, SteamError>
        + Send + Sync,
>;
```

It takes the server record because the record carries the host name, which the websocket
URI needs as its TLS server name. See [§10](#10-known-deviations-from-the-c).

**`WebSocketFactory` / `WebSocketSocket`** — the same idea one level down, so that the
library can be built and tested without a websocket dependency.

---

## 5. The library, file by file


### 5.1 `crates/steamkit/src/types/` — the value types


### 5.2 `crates/steamkit/src/base/` — message framing and the wire format


Reference for `crates/steamkit/src/types/` and `crates/steamkit/src/base/` of the Rust
SteamKit2 port. Every entry names the C# file it was ported from, the representation it
uses, and the places where the port deliberately differs from the original.

#### `crates/steamkit/src/types/` — directory overview

Nine files, all ports of `SteamKit2/SteamKit2/Types/*.cs`, in three groups: the
**identifier** types (`steam_id.rs`, `global_id.rs`, `game_id.rs`, `job_id.rs`), which pack
differently-shaped fields into the same 64-bit word through the shared `BitVector64`
helper that lives in `steam_id.rs`; the **data-container** types (`key_value.rs`,
`message_object.rs`, `pub_file.rs`); and the **depot** types (`depot_manifest.rs`,
`manifest.rs`), which together implement the legacy v4 binary depot manifest and the
current protobuf-section one. Two structural differences from the C# run through the
whole directory: the C# `SteamID`, `GlobalID`, `GameID` and `JobID` are **classes** with
inheritance (`JobID : GlobalID`, `UGCHandle : GlobalID`) while the Rust versions are
`Copy` value types that cannot inherit, so the inherited accessors are re-implemented per
type and the relationship is expressed by wrapping (`UGCHandle { global_id: GlobalID }`) or
by duplication (`GlobalID` and `JobID` share a field layout but are unrelated types); and
every C# property becomes an accessor method pair (`name()`/`set_name()`), because Rust
has no properties (the porting notes §1.4).

#### `crates/steamkit/src/types/depot_manifest.rs` — 644 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/DepotManifest.cs` (the legacy v4 binary
manifest it also parses lives in `Types/Manifest.cs`, ported to `types/manifest.rs`)
**Purpose:** Models a Steam3 depot manifest — the description of the file and chunk layout
of one depot — and reads and writes both the legacy v4 binary format and the current
protobuf-section format. It also decrypts encrypted file names with the depot's AES-256
key.
**How it works:** `ChunkData { chunk_id: Option<Vec<u8>>, checksum: u32, offset: u64,
compressed_length: u32, uncompressed_length: u32 }` mirrors `DepotManifest.ChunkData`
(the chunk id is a SHA-1, the checksum an Adler32). `FileData { file_name: String,
file_name_hash: Vec<u8>, chunks: Vec<ChunkData>, flags: u32, total_size: u64,
file_hash: Vec<u8>, link_target: Option<String> }` mirrors `DepotManifest.FileData`;
`flags` is the **raw bit pattern** rather than an `EDepotFileFlag` because one file can
carry a combination of flag bits, and protobuf-net writes `(uint)file.Flags` — the same
bits. `FileData::with_values` reproduces the C# constructor's separator normalisation:
when `encrypted` is false the name's `altDirChar` is replaced by
`Path.DirectorySeparatorChar`; when encrypted the name is left untouched.
`DepotManifest` itself holds `files: Option<Vec<FileData>>`, `filenames_encrypted: bool`,
`depot_id: u32`, `manifest_gid: u64`, `creation_time: SystemTime`,
`total_uncompressed_size`/`total_compressed_size: u64` and `encrypted_crc: u32`.
`deserialize(&[u8])` drives `internal_deserialize`, which loops reading a `u32` magic:
`0x1634_9781` (`Steam3Manifest::MAGIC`) selects the v4 binary manifest, which is parsed
by `crate::types::manifest::Steam3Manifest` and followed by a terminating magic that must
repeat the same value (the v4 format has no separate sections, so the function returns
immediately); `0x71F6_17D0`, `0x1F4_812BE` and `0x1B81_B817` select the protobuf
*payload*, *metadata* and *signature* sections, each a `u32` length followed by that many
bytes decoded with `prost` into `ContentManifestPayload` / `ContentManifestMetadata` /
`ContentManifestSignature` from `steamkit_protos`; `0x32C4_15AB` ends the manifest; any
other value is `SteamError::Malformed("Unrecognized magic value {other:X} in depot
manifest.")`. All three protobuf sections must be present, otherwise
`"Missing ContentManifest sections required for parsing depot manifest"`. The metadata is
parsed **before** the payload because `parse_protobuf_manifest_payload` needs
`self.filenames_encrypted` (already set by the metadata parse) to decide whether to
normalise directory separators — the C# orders the two calls the same way.
`serialize(&mut dyn Write)` writes the payload section, the metadata section, an **empty**
signature section and the EOF marker; the CRC is `crc32fast::hash` over
`i32 length LE || payload bytes` (the length prefix is part of the CRC input, exactly as
in the C#). When `filenames_encrypted` the CRC goes into `crc_encrypted` and `crc_clear`
is 0; otherwise `crc_encrypted` keeps the stored value and `crc_clear` receives the CRC.
The filename SHA-1 is `SHA1(lower(name with '/' replaced by '\\'))` when the names are
plain, and the stored `file_name_hash` verbatim when they are encrypted.
`decrypt_filenames(&mut self, key: &[u8]) -> bool` is a no-op returning `true` when the
names are not encrypted, requires a 32-byte key, decrypts every `file_name` and every
non-empty `link_target`, then sorts the file list with `cmp_ordinal_ignore_case` (the C#
`Files.Sort((f1, f2) => StringComparer.OrdinalIgnoreCase.Compare(...))`) and clears
`filenames_encrypted`. `decrypt_name` performs the actual work: strip whitespace, base64
decode, treat the first 16 decoded bytes as the **encrypted IV**, derive the real IV from
them with AES-256 **ECB** (no padding), decrypt the remainder with AES-256 **CBC** +
PKCS#7 using that derived IV, trim the plaintext's trailing NUL, replace every
`altDirChar` byte with `Path.DirectorySeparatorChar`, and decode the result as UTF-8.
**Notes and gotchas:** *AES key/IV ordering* — the C# does
`aes.DecryptEcb(encryptedFilename[..iv.Length], iv, PaddingMode.None)` to **derive** the
IV and then `aes.DecryptCbc(encryptedFilename[iv.Length..], iv, bufferDecrypted,
PaddingMode.PKCS7)`. The IV is therefore not the first 16 ciphertext bytes in the clear:
it is the first 16 bytes *ECB-encrypted under the same key*. The Rust reproduces this
exactly (`Aes256::new_from_slice(key)` → `BlockDecrypt::decrypt_block` on
`decoded[..16]` → `cbc::Decryptor::<Aes256>::new_from_slices(key, &iv)`). An earlier port
version passed the key and IV in the wrong order and is listed as defect #4 in the
test-suite bug table in §12 of this guide. *Strict vs lenient base64* — the C# uses
`Convert.TryFromBase64Chars`, which **ignores whitespace**; real encrypted names in a
manifest carry a trailing newline, and the `base64` crate's `BASE64_STANDARD` engine is
strict, so the port filters all whitespace out before decoding. Without that filter every
real depot manifest fails to decrypt. *The null-terminator trim* — the C# tests
`bufferDecrypted[filenameLength] == 0`, i.e. the byte **one past** the plaintext, which
is whatever the `ArrayPool`-rented buffer happened to hold there; the port reads the
intent ("trim the ending null byte") as trimming the plaintext's own trailing NUL and
checks `buffer[plaintext_len - 1]`. This was the third half of defect #4. The C# throws
`InvalidDataException`/`EndOfStreamException`; the Rust returns `SteamError::Malformed` or
an io error. `load_from_file` returns `Ok(None)` when the file does not exist (the C#
returns `null`) but propagates other io errors, as the C# does. `CreationTime` is a
`SystemTime`; the C# uses a `DateTime` of unspecified kind via
`DateUtils.DateTimeFromUnixTime`. `serialize()` builds the payload protobuf **once** and
reuses the bytes; the C# serialises it twice (once for the CRC, once for the wire) —
same result here, but the Rust cannot desynchronise the two. `unique_chunks` uses a
`HashSet<Vec<u8>>`; the C# uses a `HashSet<byte[]>` with a comparer that hashes only the
first four bytes of the SHA-1 (a speed optimisation) but compares the whole array — same
result, different hash function.

#### `crates/steamkit/src/types/game_id.rs` — 179 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/GameID.cs`
**Purpose:** `GameID` — the 64-bit handle for a Steam app, a mod, an application shortcut
or a peer-to-peer file — and `GameType`, which classifies those 64 bits.
**How it works:** `GameID { gameid: BitVector64 }` uses the same 64-bit field store as
`SteamID` and `GlobalID` (defined as `pub(crate) struct BitVector64` in
`types/steam_id.rs`). The layout is: bits 0..24 = app id (`0x00FF_FFFF`), bits 24..32 =
app type (`0xFF`), bits 32..64 = mod id (`0xFFFF_FFFF`). `from_mod(n_app_id, mod_path)`
sets the base app id, type `GameMod` and mod id = CRC-32 of the UTF-8 bytes of
`mod_path`. `from_shortcut(exe_path, app_name)` sets app id 0, type `Shortcut` and mod id
= CRC-32 of `exe_path` and `app_name` concatenated with no separator. `set_mod_id` writes
bits 32..64 **and additionally sets bit 63** (`gameid[63, 0xFF] = 1`), the "this is a
mod/shortcut" marker. `app_type()` maps bits 24..32 through `GameType::from_raw`.
`crc32_of` is `crc32fast::hash`, the same digest as the C#
`System.IO.Hashing.Crc32.HashToUInt32`. The Rust also implements `From<GameID> for u64`
and `From<GameID> for String`, mirroring the C# implicit operators to `ulong` and
`string`.
**Notes and gotchas:** `GameType::from_raw` is **not** a faithful cast. The C# does
`(GameType)gameid[24, 0xFF]`, so a raw value of 4..255 becomes an *undefined* enum value
that equals none of the four named members, and `IsSteamApp` is therefore `false`. The
Rust maps every unknown raw value to `GameType::App`, so `is_steam_app()` returns `true`
for a raw-4 id where the C# returns `false`. The doc comment on `from_raw` claims this
"mirrors the C# behaviour of casting the raw field"; it does not. Only raw 0..3 occur in
practice. `set_mod_id` sets bit 63 *after* writing the mod id; the C#'s single indexer
assignment masks the value to 32 bits and then ORs bit 63 into the same word — the two
produce the same bit pattern. `from_app_id(i32)` sign-extends: Rust's `i32 as u64` is the
same reinterpretation as the C#'s `(ulong)nAppID` applied to a negative `int`. The C#
`GameID` is a class with null-aware `==`/`!=` operators; the Rust is a `Copy` value type.

#### `crates/steamkit/src/types/global_id.rs` — 168 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/GlobalID.cs` (the same C# file also declares
`UGCHandle`)
**Purpose:** `GlobalID` — a 64-bit globally unique id with the Steam "server start time /
process / box" fields packed into it — and `UGCHandle`, the UGC-specific handle built on
the same layout.
**How it works:** `GlobalID { gid_bits: BitVector64 }` with the layout: bits 0..20 =
sequential count (`0xFFFFF`), bits 20..50 = start time in seconds since the Steam epoch
(`0x3FFF_FFFF`), bits 50..54 = process id (`0xF`), bits 54..64 = box id (`0x3FF`).
`GlobalID::new()` is `u64::MAX` — the C# `GlobalID()` delegates to `this(ulong.MaxValue)` —
while `from_value(gid)` sets the raw value. `start_time()` returns a `SystemTime` =
`UNIX_EPOCH + (1_104_537_600 + raw_seconds)`; 1_104_537_600 is the unix timestamp of
2005-01-01T00:00:00Z, the Steam epoch the C# uses as `new DateTime(2005, 1, 1)`.
`set_start_time(SystemTime)` converts to unix seconds, subtracts the Steam epoch and
writes the low 30 bits. `UGCHandle { global_id: GlobalID }` wraps rather than inherits
(the C# `UGCHandle : GlobalID`) and exposes `global_id()` to reach the inner value.
**Notes and gotchas:** **`GlobalID::default()` is 0, `GlobalID::new()` is `u64::MAX`.** The
Rust `Default` is derived (an all-zero `BitVector64`), while the C# parameterless
constructor initialises to `ulong.MaxValue`. The same split exists for `JobID`
(`default()` = 0, `new()`/`INVALID` = `u64::MAX`), `UGCHandle` and `PublishedFileID`
(`default()` = 0, `new()` = `u64::MAX`). `Default::default()` and `new()` are therefore
*not* interchangeable for these types — always use the explicit constructor. The C#
`StartTime` getter is `new DateTime(2005, 1, 1).AddSeconds(startTime)`, a `DateTime` with
`Kind = Unspecified`, i.e. 2005-01-01 in *local* wall-clock time; the Rust anchors the
same instant at UTC, so the two agree only on a UTC machine. The C# setter truncates
`(value - epoch).TotalSeconds` toward zero and casts to `uint` unchecked; the Rust
computes an `i64` delta and reinterprets it with `as u64`, which wraps like the C#'s
unchecked cast, then masks to 30 bits. The C# `GlobalID` is a class with null-aware
`==`/`!=`; the Rust is a `Copy` value type and implements neither `PartialOrd` (nor does
the C#).

#### `crates/steamkit/src/types/job_id.rs` — 515 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/JobID.cs` (the same C# file also declares
the abstract `AsyncJob`, the sealed generic `AsyncJob<T>` and `AsyncJobMultiple<T>`)
**Purpose:** `JobID` — the id of a network task — plus the awaitable job machinery: a job
that is registered with the client, completed by a matching callback, kept alive by
heartbeats and cancelled on timeout.
**How it works:** `JobID(u64)` is the C# `JobID : GlobalID`, i.e. the same 64-bit layout
(sequential count bits 0..20, start time bits 20..50, process id bits 50..54, box id
bits 54..64). The Rust does **not** inherit: `JobID` is a standalone newtype that
re-implements the four accessors. `sequential_count()` is `(self.0 >> 0) & 0xF_FFFF`,
`process_id()` is `(self.0 >> 50) & 0xF`, `box_id()` is `(self.0 >> 54) & 0x3_FF`, and
`start_time()` is `(self.0 >> 20) & 0x3_FFFF_FFFF` — the **raw** seconds count, *not* a
`SystemTime`. `JobID::INVALID = JobID(u64::MAX)` reproduces the C# `JobID.Invalid = new
JobID()`, which goes through `GlobalID()` → `ulong.MaxValue`.

`AsyncJob<T = AnyCallback>` is the awaitable job. Its fields are `job_id`, `timeout:
`Arc<Mutex<Duration>>` (default 10 s; the manager `JobEntry` shares it, so `set_timeout` changes the enforced deadline), `job_start: Instant`, `sender:
Option<oneshot::Sender<JobResult>>`, `receiver: JobReceiver` and
`PhantomData<fn() -> T>`; `JobResult = Result<Box<dyn CallbackMsg>, SteamError>` and
`AnyCallback` is the marker type of the untyped job. `new(job_id)` creates the oneshot
channel and starts the clock — it does **not** register with a client: the caller hands
the sending half to the job manager through `take_sender()`. `is_timedout()` is
`job_start.elapsed() >= timeout`; `heartbeat()` adds 10 s to the timeout.
`add_result(callback)` sends `Ok(callback)` through the channel and returns `true`
(the C# `AsyncJob<T>.AddResult` "always completes the set"). `set_failed(due_to_remote_
failure)` sends `Err(SteamError::JobFailed)` or `Err(SteamError::Timeout(timeout))` — the
C# `TrySetException(new AsyncJobFailedException())` versus `TrySetCanceled()`.
`to_task()` (typed, `T: CallbackMsg`) yields a `TypedJobReceiver<T>`; `wait()` awaits it.
`JobReceiver` is the untyped awaiting side and maps a dropped sender to
`SteamError::Cancelled`. `TypedJobReceiver<T>` is the typed awaiting side: it downcasts
the `Box<dyn CallbackMsg>` to `T` through `Any` and returns
`SteamError::Malformed("callback message does not match the expected type")` on a
mismatch.

`AsyncJobMultiple<T: CallbackMsg>` is the multi-result job. `finish_condition: Box<dyn
FnMut(&T) -> bool + Send + Sync>` is the C# `Predicate<T>`. `add_result` downcasts the
callback (a type mismatch returns `false` and the callback is dropped), evaluates the
finish condition, pushes the callback into `results`, and — if the condition holds —
sends `Ok(ResultSet { complete: true, failed: false, results: Some(..) })`; otherwise it
calls `heartbeat()` and returns `false`. `set_failed` behaves differently depending on
the accumulated results: with none it fails the job (`JobFailed` or `Timeout`), with a
partial set it **completes** the job with `ResultSet { complete: false, failed:
due_to_remote_failure, results: Some(..) }` so the consumer can decide what to do with an
incomplete set. `ResultSet<T> { complete: bool, failed: bool, results: Option<Vec<T>> }`
is the C# nested `AsyncJobMultiple<T>.ResultSet`, whose `ReadOnlyCollection<T>?` is
nullable. The file ends with the aliases the job manager uses: `JobState = AsyncJob`,
`JobHandle<T> = AsyncJob<T>`, `TypedJob<T> = TypedJobReceiver<T>`, `JobGroup<T> =
AsyncJobMultiple<T>`, `MultiJob<T> = MultiJobReceiver<T>`.

**The rename, and why.** The C# file `Types/JobID.cs` declares **four** types: `JobID`,
the abstract base class `AsyncJob`, the sealed generic `AsyncJob<T>` and
`AsyncJobMultiple<T>`. The name collision is between the base class `AsyncJob` and the
generic `AsyncJob<T>`: Rust forbids a non-generic type and a generic type with the same
name in one module, so `struct AsyncJob` and `struct AsyncJob<T>` cannot coexist. The
resolution (recorded in the porting notes §1.1 and in §12 of this guide) is to collapse the pair
into **one** generic type with a default type parameter: `AsyncJob<T = AnyCallback>`.
`AsyncJob` (untyped) is `AsyncJob<AnyCallback>` and `AsyncJob<T>` is the typed wrapper;
Rust's default-type-parameter fallback makes both spellings name valid types in the same
module. The "untyped base" is therefore not a separate type but a distinguished
instantiation, and the marker type `AnyCallback` is what makes the two spellings
different instantiations rather than the same one.
`AsyncJobMultiple<T>` is not a specialisation of `AsyncJob<T>` in the C# either — it is a
sibling that derives from the abstract base and re-implements `AddResult`/`SetFailed`.
Rust has no inheritance, so the Rust `AsyncJobMultiple<T>` is an independent struct that
repeats the timeout/heartbeat code and owns its own channel. Because Rust cannot express
"an abstract base class with an abstract method", the *typing* moved from the producing
side to the consuming side: the C# casts `(T)callback` inside `AddResult` and throws
`InvalidCastException` on a mismatch, whereas the Rust stores `Box<dyn CallbackMsg>` and
downcasts in `TypedJobReceiver<T>::wait`.

**Notes and gotchas:** **`JobID::default()` is 0, `JobID::INVALID` is `u64::MAX`.** The
`Default` impl is derived, so `JobID::default()` is *not* the C# `new JobID()`; use
`JobID::INVALID` for "invalid". `JobID::start_time()` returns the raw 30-bit seconds
count, not a `SystemTime` — it is **not** interchangeable with
`GlobalID::start_time()` even though both describe the same 30-bit field. The C#
`AsyncJob` constructor takes `(SteamClient client, JobID jobId)` and the derived class
calls `RegisterJob(client)`; the Rust constructor takes only the job id and registration
is `take_sender()` → the job manager. The C# `AsyncJob` declares `internal abstract bool
AddResult(CallbackMsg)` and `internal abstract void SetFailed(bool)`; the Rust
`AsyncJob<T>` implements both concretely for **every** `T` including `AnyCallback`, so the
untyped job also completes on the first callback. `AsyncJobMultiple::add_result` returns
`false` and silently drops a callback whose type does not match `T`, where the C# throws
`InvalidCastException` — a mismatched callback is invisible in Rust. `AsyncJobMultiple::
add_result` pushes the callback into `results` *after* evaluating the finish condition,
while the C# pushes it first; the predicate only ever sees the callback, so the outcome
is the same, but a panicking predicate loses the callback in Rust. The timeout starts at
10 s and grows by 10 s per heartbeat, exactly as the C# `Timeout += TimeSpan.FromSeconds(
10)`. The C# uses `TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously
)`; `tokio::sync::oneshot` gives the same "the continuation does not run inline"
property. `ResultSet.results` is an `Option<Vec<T>>` because the C# collection is
nullable. `SteamError::Cancelled` (sender dropped) has no direct C# equivalent — in C#
the `Task` would simply never complete.

#### `crates/steamkit/src/types/key_value.rs` — 1137 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/KeyValue.cs` (the same C# file also declares
the private `KVTextReader`)
**Purpose:** `KeyValue` — the recursive string-keyed container Valve uses for app info,
depots, published files and many other payloads — in both the binary (VDF1) and textual
(VDF2) encodings.
**How it works:** `KeyValue { name: Option<String>, value: Option<String>, children:
Vec<KeyValue> }` is the C# `public class KeyValue` with `Name`/`Value`/`Children`. The
private `KeyValueType` enum mirrors the C# private nested `Type` enum: None 0, String 1,
Int32 2, Float32 3, Pointer 4, WideString 5, Color 6, UInt64 7, End 8, Int64 10,
AlternateEnd 11 — note the gap at 9. `KeyValue::invalid()` returns a `OnceLock`-held
singleton, the counterpart of the C# `public readonly static KeyValue Invalid`.
`get(key)` and `set(key, value)` are the C# indexer: the lookup is **ordinal,
case-insensitive** (`ordinal_ignore_case_eq`), the first match wins, and `set` removes the
first matching child, renames the incoming value to `key` and appends it.

The typed accessors `as_unsigned_byte` / `as_unsigned_short` / `as_integer` /
`as_unsigned_integer` / `as_long` / `as_unsigned_long` / `as_float` / `as_boolean` /
`as_enum` are the C# `As*` family. They parse the *string* `Value` and return the caller's
default on a parse failure. `as_boolean` parses an integer and returns `value != 0` (the
C# `AsBoolean` also parses an `int`, not a boolean token). `as_float` accepts a thousands
separator because the C# uses `NumberStyles.Float | NumberStyles.AllowThousands` with the
invariant culture and `parse_float` strips `,` before parsing. `as_enum::<T: FromKeyValue>`
is the Rust stand-in for the C# `Enum.TryParse<T>`: the `FromKeyValue` trait accepts
either the *name* of a variant or its *numeric* value. `EDepotFileFlag` has a
hand-written implementation that matches the variant names case-insensitively through
`flag_name` before falling back to the numeric form.

Binary reading is `try_read_as_binary(&mut dyn Read) -> bool` → `try_read_as_binary_core`.
It reads a type byte; `End`/`AlternateEnd` end the current level; anything else is
followed by a NUL-terminated UTF-8 name and then the type-specific payload. `None`
opens a nested group (read recursively, appended to the current node's children);
`String` reads a NUL-terminated UTF-8 value; `Int32`/`Color`/`Pointer` read a
little-endian `i32` rendered with `to_string()`; `UInt64` reads an LE `u64`; `Float32`
an LE `f32`; `Int64` an LE `i64`. `WideString` is unsupported: the C# logs and returns
false, the Rust emits a `tracing::warn!` and fails the parse. End of stream (the C#'s
`ReadByte()` returning -1) is a parse failure. Binary writing is `save_to_stream(stream,
as_binary)` → `recursive_save_binary_to_stream`: a node without a value writes `None`,
the NUL-terminated name, then each child's core, then `End`; a node with a value writes
`String`, the NUL-terminated name and the NUL-terminated value. The root additionally
gets a trailing `End`.

Text reading is `read_as_text(&mut dyn Read)` → `KvTextReader`. `decode_text` sniffs the
byte-order mark: `EF BB BF` (UTF-8), `FF FE` (UTF-16 LE), `FE FF` (UTF-16 BE); the mark
is stripped and the remainder decoded. Without a mark the bytes are decoded as UTF-8.
`decode_utf16` pairs the code units, maps surrogate pairs to a code point, replaces an
unpaired surrogate with `U+FFFD` and rejects an odd trailing byte with "the utf-16
document ends with an incomplete code unit". `KvTextReader::read_token` reproduces the
C# `ReadToken(out wasQuoted, out wasConditional)`: whitespace is eaten, a `/` comments
out the rest of the line, a `"` starts a quoted token in which `\`, `\n`, `\r`, `\t` are
the escape sequences, `{` and `}` are single-character tokens, and an unquoted token ends
at whitespace, `"`, `{` or `}`; a `[`…`]` group sets the `wasConditional` flag.
`recursive_load_from_buffer` reads `name value` pairs, opening a nested group on a `{`
value and closing the level on `}`. Only the **first** root object is kept — a second root
is parsed into a throwaway `KeyValue` and discarded, matching the C#, which parses further
roots into a `KeyValue` that is never handed back. Text writing is
`recursive_save_text_to_stream`: `"name"\n{\n`, then each child — a child with a value is
written as `"name"\t\t"value"` with `escape_text` applied — and finally `}\n`.
`escape_text` walks `ESCAPED_MAPPING = [('\\','\\'), ('n','\n'), ('r','\r'), ('t','\t')]`
in the C#'s insertion order and replaces each *value* with its escaped form.

**Notes and gotchas:** *BOM sniffing* — the C# `KVTextReader` derives from `StreamReader`,
whose `detectEncodingFromByteOrderMarks` defaults to `true`, so a UTF-8, a UTF-16 LE and
a UTF-16 BE document are all read correctly. The first port only stripped the UTF-8 BOM
and a UTF-16 VDF came out as mojibake; that was defect #7 in the test-suite bug table in
§12 of this guide, and the fix was `decode_text` plus `decode_utf16`. *The binary reader
keeps only the first top-level entry* — the C# `TryReadAsBinaryCore` rebinds its `current`
**parameter** after each entry, so the caller's object is mutated in place by the first
entry and then left alone while the local parameter points at a fresh `KeyValue`; the
first top-level entry therefore survives and every later one is discarded. The Rust
reproduces this with an explicit `first_entry` slot. `try_read_as_binary` also copies the
existing name and value into a `carrier` before parsing, so a parse that yields no entry
at all leaves the instance's name and value intact — the C# clears only `Children`. *The
`Invalid` singleton* — `KeyValue::invalid()` returns a `&'static KeyValue`; it is the
same shared instance the C# returns, and callers must not assume it is a live child.
*Struct vs class* — the C# `KeyValue` is a class, so `Children` is a shared
`List<KeyValue>` and the indexer mutates it in place; the Rust is a struct, so
`children_mut()` hands out `&mut Vec<KeyValue>` and copying a `KeyValue` copies its whole
subtree. *The C# indexer setter removes the first matching child while iterating and
then breaks*, which does **not** trip .NET's collection-modified check (the enumerator's
`MoveNext` is never called again), so the C# setter works; the Rust finds the index first
and removes it, which is equivalent. *`AsBoolean` parses an integer*, so `"true"` yields
the default and `"1"` yields `true`. *`AsFloat` accepts `1,234.5`* because the C# allows
thousands separators. *`AsEnum<T>`* — the C# `Enum.TryParse` accepts a member name
**case-sensitively**; the Rust `EDepotFileFlag` implementation matches names
**case-insensitively**, which is slightly more permissive. The C# `ReadAsText` throws on a
malformed document; the Rust returns `SteamError::Malformed` with the same message text
("LoadFromBuffer: missing {", "RecursiveLoadFromBuffer: got EOF or empty keyname", …).
`load_as_text` / `load_from_string` swallow the error and return `None`, matching the C#
`LoadAsText` / `LoadFromString`. The C# `LoadFromFile` opens with `FileShare.ReadWrite`;
the Rust `tokio::fs::read` has no sharing flag. `eat_cpp_comment` treats a **single** `/`
as a line comment, matching Valve's own reference implementation (the C# comment says so
explicitly). The C# text reader caps an unquoted token at 1024 characters and throws
"ReadToken overflow"; the Rust has no cap. `read_null_term_string` uses
`String::from_utf8_lossy`, so invalid UTF-8 becomes `U+FFFD` instead of failing the
parse — the same as the C# `Encoding.UTF8` decoder.

#### `crates/steamkit/src/types/manifest.rs` — 306 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/Manifest.cs` (the `internal sealed class
Steam3Manifest`)
**Purpose:** the *legacy v4* binary depot manifest format, plus the shared little-endian
byte reader and the unix-time helpers that the depot manifest code builds on.
**How it works:** `ByteReader<'a>` is a cursor over a byte slice with LE `read_u32` /
`read_u64`, `read_bytes` (a **short** read, like `BinaryReader.ReadBytes`), a
NUL-terminated lossy-UTF-8 `read_null_term_string` and `position()`.
`date_time_from_unix_time(u64) -> SystemTime` and `date_time_to_unix_time(SystemTime) ->
u64` are the C# `DateUtils.DateTimeFromUnixTime` / `DateTimeToUnixTime`.
`Steam3Manifest` is the v4 header: `magic` (`0x1634_9781`), `version`, `depot_id`,
`manifest_gid`, `creation_time`, `are_file_names_encrypted`, `total_uncompressed_size`,
`total_compressed_size`, `chunk_count`, `file_entry_count`, `file_mapping_size`,
`encrypted_crc`, `decrypted_crc`, `flags` and `mapping: Vec<Steam3ManifestFileMapping>`.
`deserialize(reader)` reads the version first — the magic has already been consumed by
`DepotManifest::internal_deserialize` — and rejects anything but version 4. The mapping
section is then read until `file_mapping_size` **bytes** have been consumed, measuring
each entry with the stream position; an entry that consumes more than the remaining
budget is rejected as `SteamError::Malformed`. `Steam3ManifestFileMapping` is `file_name`
(NUL-terminated), `total_size`, `flags`, `hash_content` (20 bytes), `hash_file_name` (20
bytes), `num_chunks` and `chunks`; `Steam3ManifestChunk` is `chunk_gid` (20 bytes),
`checksum`, `offset`, `decompressed_size` and `compressed_size`.
**Notes and gotchas:** the C# `Steam3Manifest` is `internal` to the assembly; the Rust type
is `pub(crate)`. The C# throws `NotImplementedException($"Only version {CURRENT_VERSION}
is supported.")`; the Rust returns `SteamError::Malformed("Only version 4 is
supported.")`. `CreationTime` is read as a **32-bit** unix time (`read_u32()? as u64`),
matching the C#. The mapping loop treats `file_mapping_size` as a byte budget, not a
count: the C# `for (uint i = FileMappingSize; i > 0; )` subtracts the bytes each entry
consumed. The Rust does the same and returns `SteamError::Malformed("data is not a valid
steam3 manifest: file mapping exceeds the declared size.")` when an entry overruns the
budget, so a malformed manifest cannot wrap the counter or spin. The magic is deliberately
**not** verified inside `Steam3Manifest::deserialize`
(the C# comments the check out to avoid seeking); `DepotManifest::internal_deserialize`
has already matched `Steam3Manifest::MAGIC`. The C# reads the SHA-1 hashes with
`ds.ReadBytes(20)`, which returns fewer than 20 bytes at the end of the stream without
throwing; the Rust `read_bytes(20)` does the same.

#### `crates/steamkit/src/types/message_object.rs` — 52 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/MessageObject.cs`
**Purpose:** a thin wrapper around a `KeyValue` that Steam servers use to send free-form
payload blobs; it reads and writes the binary VDF form.
**How it works:** `MessageObject { key_values: KeyValue }`. `new()` creates a `KeyValue`
named `"MessageObject"` (the C# `new KeyValue("MessageObject")`); `with_key_values`
adopts an existing `KeyValue`. `read_from_stream(&mut dyn Read) -> bool` delegates to
`KeyValue::try_read_as_binary`; `write_to_stream(&mut dyn Write)` delegates to
`KeyValue::save_to_stream(stream, true)` — the binary form.
**Notes and gotchas:** the C# `MessageObject.KeyValues` has a private setter and the class
is mutated in place; the Rust exposes `key_values()` and `key_values_mut()`. There is no
text form here. Because `KeyValue::try_read_as_binary` replaces the whole `KeyValue`
(including its name) with the parsed root, reading into a `MessageObject` loses the
`"MessageObject"` name — the C# behaves the same way, because `TryReadAsBinary` mutates
`this` in place and `TryReadAsBinaryCore` overwrites `current.Name`.

#### `crates/steamkit/src/types/pub_file.rs` — 56 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/PubFile.cs`
**Purpose:** `PublishedFileID` — the handle to a Steam Workshop item — plus the
`UInt64Handle` marker that gives the handle its type safety.
**How it works:** `trait UInt64Handle: Copy + PartialEq + Eq + Hash { fn value(&self) ->
u64; }` is the Rust counterpart of the C# `public abstract class UInt64Handle` with its
`protected ulong Value`. It is a trait rather than a base class because the C# class can
only be used through its subclasses and Rust has no implementation inheritance.
`PublishedFileID(u64)` implements the trait; `new()` is `u64::MAX` (the C#
`PublishedFileID(ulong fileId = ulong.MaxValue)`), `from_value(file_id)` sets it
explicitly, and `From<PublishedFileID> for u64` / `From<u64> for PublishedFileID`
reproduce the C# implicit operators.
**Notes and gotchas:** the C# `PublishedFileID` also declares `operator ==` / `!=` on the
concrete type and inherits `Equals` / `GetHashCode` / `ToString` from `UInt64Handle`; the
Rust derives `PartialEq` / `Eq` / `Hash` / `Debug` and implements `Display` as the
decimal value. `PublishedFileID::default()` is **0** while `PublishedFileID::new()` is
`u64::MAX` — the derived `Default` is not the C# default constructor. The C# file's
`#pragma warning disable 0660/0661` (an `==` operator without an `Equals` override) has
no Rust counterpart, because the derived `PartialEq`/`Hash` cover the same ground.

#### `crates/steamkit/src/types/steam_id.rs` — 820 lines
**C# counterpart:** `SteamKit2/SteamKit2/Types/SteamID.cs` (the same C# file also declares
the `BitVector64` helper class)
**Purpose:** `SteamID` — the 64-bit identifier of an account, clan, chat group, lobby or
game server — with parsing and rendering of both the Steam2 (`STEAM_0:1:1234`) and Steam3
(`[U:1:1234]`) textual forms. The file also hosts the `BitVector64` bit-field helper that
`GlobalID` and `GameID` reuse.
**How it works:** `BitVector64 { data: u64 }` with `get(bitoffset, valuemask)` =
`(data >> offset) & mask` and `set(bitoffset, valuemask, value)` = `(data & !(mask <<
offset)) | ((value & mask) << offset)`. The C# indexer `this[uint bitoffset, ulong
valuemask]` does exactly this; the Rust uses `wrapping_shr` / `wrapping_shl`, which mask
the shift count to 63 just as .NET does. `SteamID(u64)` stores the raw 64-bit value with
the layout: bits 0..32 = account id (`0xFFFF_FFFF`), bits 32..52 = account instance
(`0x000F_FFFF`, 20 bits), bits 52..56 = account type (4 bits), bits 56..64 = account
universe (8 bits).

`ChatInstanceFlags` holds `Clan = (0x000FFFFF + 1) >> 1 = 0x80000`, `Lobby = 0x40000`
and `MMSLobby = 0x20000`. `has_flag` is the `Enum.HasFlag` equivalent and `BitOr`
produces the raw `u32` bit pattern.

`from_parts(account_type, universe, account_id)` delegates to `set`, which writes the
account id, universe and type and then sets the instance to `ALL_INSTANCES` (0) for
`Clan` and `GameServer` and to `DESKTOP_INSTANCE` (1) for every other type.
`from_parts_instanced` delegates to `instanced_set`, the same with an explicit instance.
`set_from_string(&str, universe)` parses the Steam2 form: `STEAM_<0-4>:<0-1>:<digits>`,
matched case-insensitively and **not anchored** (the leftmost match anywhere in the input
wins). It then sets universe = the *caller-supplied* universe, instance = 1, type =
`Individual` and account id = `(accId << 1) | authServer`, returning `false` when the
string does not match. `set_from_steam3_string(&str)` parses
`[<type>:<0-4>:<account>(:<instance>)?]`, falling back to the parenthesised instance form
`[<type>:<0-4>:<account>(<instance>)]`. The instance defaults to 0 for the type characters
`g`, `T`, `c` and `L` and to 1 otherwise. Then `c` ORs in `ChatInstanceFlags::Clan` and
sets type `Chat`; `L` ORs in `ChatInstanceFlags::Lobby` and sets type `Chat`; `i` sets
type `Invalid`; every other character is looked up in the `AccountTypeChars` table. The
parsers are hand-written byte scanners (`parse_steam2_id`, `parse_steam3_id`,
`parse_steam3_at`, `parse_steam3_tail`) that reproduce the C# regular expressions,
including the greedy-with-backtracking behaviour of the account-id group.

`account_type()` and `account_universe()` map the raw 4-bit / 8-bit fields onto the
generated `EAccountType` / `EUniverse` enums; unknown raw values (11..15 and 5..255) are
reported as `Invalid`. `is_valid()` implements the C# `IsValid`: the account type must lie
in `Invalid..=AnonUser`, the universe in `Invalid..=Dev`, an `Individual` needs a non-zero
account id and an instance ≤ `WEB_INSTANCE` (4), a `Clan` needs a non-zero account id and
instance 0, and a `GameServer` needs a non-zero account id. `get_static_account_key()` is
`(universe << 56) + (type << 52) + account_id` — the key that groups accounts differing
only in their instance.

`render(steam3: bool)` dispatches to `render_steam3()` or `render_steam2()`.
`render_steam3` takes the type character from the `AccountTypeChars` table, except that a
`Chat` id renders as `c` when the instance has the `Clan` bit and as `L` when it has the
`Lobby` bit. The instance is rendered when the type is `AnonGameServer` or `Multiseat`, or
when the type is `Individual` and the instance is not `DESKTOP_INSTANCE`.
`render_steam2` returns `STEAM_<universe>:<id & 1>:<id >> 1>` for `Invalid` and
`Individual` — the universe digit is `0` for `Public` and lower — and the **decimal
64-bit value** for every other account type. `to_chat_id()` performs the clan → chat
conversion (instance = `ChatInstanceFlags::Clan`, type = `Chat`); `try_to_chat_id()` is
the checked variant. `try_get_clan_id()` is the inverse and succeeds only when the id is
a chat account whose instance is *exactly* `ChatInstanceFlags::Clan`.

**Notes and gotchas:** ***`Convert.ToString(this)` overload resolution*** — the C#
`RenderSteam2()`'s `default:` branch is `return Convert.ToString( this );`.
`Convert.ToString` has both an `object` overload and a `ulong` overload, and `SteamID`
has an implicit user-defined conversion to `ulong`. Under the C# "better conversion from
expression" rules a user-defined implicit conversion beats a standard implicit
(reference) conversion, so the call binds to `Convert.ToString(ulong)` and renders the
**numeric id** — not `SteamID.ToString()`, which would be the Steam3 form. The Rust
`render_steam2` therefore returns `self.0.to_string()`. The first port fell back to the
Steam3 rendering for every type but `Invalid`/`Individual`; that was defect #8 in the
test-suite bug table in §12 of this guide. *Raw fields versus enum-mapped fields* —
`is_valid`, `is_game_server_account`, `is_chat_account` and the other predicates compare
the **raw** 4-bit / 8-bit fields (`account_type_raw()`, `account_universe_raw()`), not the
enum-mapped values. The generated `EAccountType` cannot represent raw 11..15 and
`EUniverse` cannot represent raw 5..255; the mapper folds them to `Invalid` (0), and a
raw universe of 5 would then compare as `0`, which passes the `universe <= Invalid`
check and is rejected only by accident. The raw comparison is the only one that is right
by construction — use `account_type_raw()` / `account_universe_raw()` for any range
check. *The account universe of a Steam2 string is ignored* — `set_from_string` parses
the universe digit out of `STEAM_0:1:1234` and then uses the **caller-supplied** universe,
exactly like the C# (which captures the group and never reads it).
`set_from_string("STEAM_0:1:1", EUniverse::Dev)` therefore produces a `Dev` id. *The
regular expressions are not anchored* — the C# `Steam2Regex` / `Steam3Regex` have no `^`
or `$`, so `SetFromString("hello STEAM_0:1:1 world")` succeeds and uses the first match;
the Rust scanners reproduce that by scanning left to right. *The Steam3 account-id group
backtracks* — `(?<account>[0-9]+)(:(?<instance>[0-9]+))?` is greedy, so `[U:1:123:4]`
gives account id `123` and instance `4`, while `[U:1:1234]` gives account id `1234`; when
the optional instance group cannot match, the engine backtracks and hands the digits to
the account group. The Rust tries progressively shorter account ids until the tail
matches. *The fallback form is tried only when the primary form fails* — `[U:1:123(4)]`
parses through `Steam3FallbackRegex`, `[U:1:123:4]` through `Steam3Regex`, and a string
containing both uses the primary form wherever it matches. *The Steam2 regex is
case-insensitive* (`RegexOptions.IgnoreCase`), so `steam_0:1:1` parses; the Steam3
regexes are case-sensitive. *`EAccountType::ConsoleUser` has no character* — the C#
`AccountTypeChars` dictionary has ten entries and no `ConsoleUser`, so a console-user id
renders as `i` (the unknown character); the Rust `account_type_char` maps raw 9 to
`UNKNOWN_ACCOUNT_TYPE_CHAR` for the same reason, and parsing an `i` sets the type to
`Invalid`, matching the C#'s `type == UnknownAccountTypeChar` branch. *The chat instance
flags are bit positions inside the 20-bit instance field*, not instance numbers:
`Clan` = 0x80000, `Lobby` = 0x40000, `MMSLobby` = 0x20000. `try_get_clan_id` compares the
instance for **equality** with `Clan`, so a lobby chat that also carries the clan bit
does not convert back. `to_chat_id()` returns a raw `u64` and does **not** check that the
id is a clan id; `try_to_chat_id()` is the checked variant and returns
`SteamError::invalid_argument("Only Clan IDs can be converted to Chat IDs.")` — the C#
throws `InvalidOperationException` with the same text. `Display` is the Steam3 form
(`render(true)`), matching the C# `ToString() => Render()` with the default `steam3 =
true`; `Debug` delegates to `Display`. Both setters validate before writing, so a failed
parse leaves the object unchanged. `BitVector64` is `pub(crate)` — an implementation
detail shared by `steam_id.rs`, `global_id.rs` and `game_id.rs`, not part of the public
API. The Rust `SteamID` implements neither `PartialOrd` nor ordering operators; the C#
has none either.

#### `crates/steamkit/src/base/` — directory overview

Eight files, all ports of `SteamKit2/SteamKit2/Base/*.cs`, forming the message layer:
`msg_base.rs` is the payload buffer and the primitive readers/writers, `client_msg.rs` and
`packet_base.rs` are the outbound envelope and the inbound frame view, `gc/` holds the
game-coordinator variants of the same three concepts, and `generated/` holds the
machine-generated Valve enum code plus the hand-written wire-format layer. The Rust port
splits the C#'s class hierarchies apart: the C# `MsgBase` is an abstract class with a
`MemoryStream Payload` and a `BinaryReader`/`BinaryWriter` on top, and `MsgBase<THeader>`
is a generic abstract subclass that adds a `Header` property and implements `IClientMsg`,
but Rust has neither abstract classes nor generic inheritance — so the Rust `MsgBase` is a
concrete `Vec<u8>` + cursor, the `Header` lives on the concrete message types, and the
shared behaviour is expressed through the `IClientMsg` and `IPacketMsg` traits. The
protobuf body types are `prost` messages from `crates/steamkit-protos` rather than
protobuf-net classes; the porting notes §4 records the substitution and the one thing it
costs (protobuf-net's `IExtensible` has no prost equivalent, so unknown fields are not
preserved).

#### `crates/steamkit/src/base/msg_base.rs` — 479 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/MsgBase.cs` (the same C# file also declares
the `IClientMsg` interface and the abstract `MsgBase<THeader>`; the Rust `IClientMsg`
lives in `base/client_msg.rs`)
**Purpose:** the payload buffer and the primitive readers/writers that every Steam
message is built from.
**How it works:** two very similar primitives live here. `SeekOrigin` mirrors
`System.IO.SeekOrigin`. `Reader<'a>` is a forward-only cursor over a borrowed slice —
the Rust addition that lets the `SteamSerializableDeserialize` trait read without
allocating; it offers LE `read_u8/i8/u16/i16/u32/i32/u64/i64/f32/f64`,
`read_bytes` (exact, errors), `read_up_to` (short read), `read_null_term_string` (UTF-8,
lossy) and the `position()` / `remaining()` / `is_empty()` accessors.
`MsgBase { payload: Vec<u8>, position: u64 }` is the Rust counterpart of the C#
`MsgBase`'s `MemoryStream Payload` plus the `BinaryReader`/`BinaryWriter` that wrapped
it. Its API: `new(payload_reserve)`, `from_bytes(data)`, `seek(offset)`,
`seek_from(offset, origin)`, `len()`, `is_empty()`, `to_bytes()` (the **whole** buffer,
the C# `MemoryStream.ToArray`), `reader()` (a `Reader` over the part after the cursor),
the writers `write_u8/i8/u16/i16/u32/i32/u64/i64/f32/f64`, `write_bytes`, `write_string`
(UTF-8, no terminator) and `write_null_term_string`, and the readers `read_exact`
(errors), `read_bytes` (short read), `read_u8/i8/…`, `read_null_term_string`. The file
also keeps the C#-named aliases `read_int8`, `read_sbyte`, `read_uint8`, `read_byte`,
`read_int16`, `read_short`, `read_uint16`, `read_ushort`, `read_int32`, `read_int`,
`read_uint32`, `read_uint`, `read_int64`, `read_long`, `read_uint64`, `read_ulong`,
`read_single`, `read_float` and `read_double`.
**Notes and gotchas:** the C# `MsgBase` is an abstract class whose only state is the
`MemoryStream`; the Rust `MsgBase` is a concrete struct with a `Vec<u8>` and a cursor,
the model mandated by the porting contract §13.2. `write_bytes` grows the `Vec` with
**zeroes** between the old end and the new write, exactly as a growing `MemoryStream`
does — and because the write happens at the *current position*, a write past the end
zeroes the gap. `MsgBase::read_bytes` is the *short* read (the C#
`BinaryReader.ReadBytes` returns fewer bytes at the end of the stream); `read_exact` is
the Rust addition that errors instead. The two null-terminated-string readers behave
differently: `Reader::read_null_term_string` tolerates the end of the data and returns
the partial string, while `MsgBase::read_null_term_string` reads byte by byte through
`read_u8`, which **errors** at the end of the payload. The C# `Payload.ReadNullTermString
(Encoding.UTF8)` stops at the end of the stream without throwing, so the `Reader`
behaviour is the faithful one and the `MsgBase` behaviour is the stricter one.
`MsgBase::position` is a `u64` while the buffer is indexed by `usize`; the conversions
saturate. The C# exposes `Write(byte[])`, `Write(string)`, `Write(string, Encoding)` and
`WriteNullTermString`; the Rust exposes `write_bytes`, `write_string` and
`write_null_term_string` — all UTF-8, which is what the C# defaults to.

#### `crates/steamkit/src/base/client_msg.rs` — 560 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/ClientMsg.cs` (plus the `IClientMsg`
interface, which the C# declares in `Base/MsgBase.cs`)
**Purpose:** the message envelopes — `ClientMsgProtobuf<TBody>`, `ClientMsg<TBody>` and
`Msg<TBody>` — plus the `IClientMsg` trait that the dispatcher and the handler layer
program against.
**How it works:** `trait IClientMsg: Send + Sync` declares `is_proto()`, `is_extended()` (default `false`), `msg_type()`,
`session_id()` / `set_session_id()`, `steam_id()` / `set_steam_id()`,
`target_job_id()` / `set_target_job_id()`, `source_job_id()` / `set_source_job_id()` and
`serialize() -> Vec<u8>`. The C# `IClientMsg` is the same set of properties plus
`byte[] Serialize()`; the Rust turns the properties into method pairs (the property and
indexer conversion recorded in the porting notes §1.4). `is_extended()` distinguishes the 36-byte extended header from the plain one and gates the identity stamping.

`SteamProtobufHeaderOnly` is an empty `prost::Message` used as the default body type. The
C# has a *separate* non-generic class `ClientMsgProtobuf` (header only, read-only) and
the generic `ClientMsgProtobuf<TBody>` that derives from it; Rust has no inheritance, so
the pair becomes one generic type with a default type parameter:
`ClientMsgProtobuf<TBody: prost::Message + Default = SteamProtobufHeaderOnly>`.
`ClientMsgProtobuf<TBody>` holds `header: MsgHdrProtoBuf`, `body: TBody` and
`payload: MsgBase`. `new(e_msg)` / `with_payload_reserve(e_msg, reserve)` build it;
`reply(e_msg, reply_to)` sets the target job id to the source job id of the message
being answered (the C# `Header.Proto.jobid_target = msg.Header.Proto.jobid_source`);
`from_packet(&dyn IPacketMsg)` rejects a non-proto packet, reads the `MsgHdrProtoBuf`
from the raw data, decodes the body from the remaining bytes and keeps the leftovers as
the payload; `serialize()` writes the header, then `body.encode_to_vec()`, then the whole
payload buffer. `proto_header()` / `proto_header_mut()` expose the
`CMsgProtoBufHeader` shorthand (the C# `ProtoHeader` property).

`ClientMsg<TBody>` is the struct-backed variant with an `ExtendedClientMsgHdr`; its
`is_proto()` is false and the session id and SteamID come from the header.
`Msg<TBody>` is the struct-backed variant with a plain `MsgHdr`; its session id and
SteamID are **not** in the header and are kept as separate fields on the message
(the C# `Msg<TBody>` has them as auto-properties). `SteamIDRaw` is a helper trait with
`raw_u64()`, used to write a `SteamID` into a header field.

**Notes and gotchas:** *the C# `ClientMsgProtobuf.Serialize()` throws
`NotSupportedException`* ("ClientMsgProtobuf is for reading only. Use
ClientMsgProtobuf<T> for serializing messages."). The Rust `IClientMsg::serialize` has
no error channel, so the header-only variant serialises the header plus an empty body
instead of throwing — callers that need the C# behaviour must check the body type
themselves. *`SteamIDRaw::raw_u64` rebuilds the id from the public fields* using the C#
`SteamID.Set()` instance rules: instance 0 for `Clan`/`GameServer`, 1 otherwise. Any
SteamID whose 20-bit instance is not the default — a chat SteamID (`Clan` = 0x80000,
`Lobby` = 0x40000), a console instance (2) or a web instance (4) — is therefore
**re-encoded incorrectly** when it is written into a message header, and an account type
outside the generated enum (raw 11..15) is mangled too, because `account_type()` folds it
to `Invalid` (0). The file's own doc comment flags this ("Follow-up required: the
`types` wave should add `pub fn value(&self) -> u64` to `SteamID`"); the accessor now
exists as `SteamID::convert_to_u64()` in `types/steam_id.rs` and should be used instead.
*The message type is explicit for `ClientMsg`, derived for `Msg`.* The C# `ClientMsg<TBody>` and
`Msg<TBody>` **derive** the message type from the body (`Header.SetEMsg(Body.GetEMsg())`). The
Rust `ClientMsg::new(e_msg)` still takes it explicitly, but `Msg::new()` derives it from
`TBody::get_emsg()` like the C#, and `Msg::reply(reply_to)` takes no EMsg either.
(The game-coordinator variant in `base/gc/client_msg_gc.rs` still derives it from the body, as the C# does.) *The header
is parsed twice on the packet path* — `from_packet` re-parses the `MsgHdrProtoBuf` from
the raw bytes to find the body offset, whereas the C# copies the already-parsed
`packetMsg.Header` and starts the body at `packetMsg.BodyOffset`. The bytes are the
same; the Rust simply does the work twice. *The payload is normally empty* — the C#
creates the body `MemoryStream` over `data[offset..]` and copies `ms.Length -
ms.Position` into `Payload`, seeking the payload back to 0; the Rust decodes the body
with `prost` from the remaining slice and stores the **unconsumed** remainder. Because a
protobuf message on the Steam wire carries no length, `prost` consumes the whole
remainder, so the payload is empty — the same as the C#. *`serialize()` appends the
whole payload buffer*, not the part after the cursor, matching `MemoryStream.WriteTo`.
`IClientMsg::steam_id()` returns `Option<SteamID>`; the C# property is `SteamID?` and
its setter throws `ArgumentNullException` on `null`.

#### `crates/steamkit/src/base/packet_base.rs` — 217 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/PacketBase.cs`
**Purpose:** the immutable view of a frame that arrived from the network — the header
plus the offset at which the body starts — so the dispatcher can route a frame before
its body is decoded.
**How it works:** `trait IPacketMsg: Send + Sync` declares `is_proto()`, `msg_type()`,
`target_job_id()`, `source_job_id()` and `get_data() -> &[u8]`. The C# `IPacketMsg`
returns `ulong` for the two job ids; the Rust returns `JobID`.
`PacketClientMsgProtobuf`, `PacketClientMsg` and `PacketMsg` each own the raw
`Vec<u8>`, parse their header (`MsgHdrProtoBuf`, `ExtendedClientMsgHdr`, `MsgHdr`
respectively) to obtain the job ids, and record `body_offset` as the reader position
right after the header. `get_msg_type_with_null_check(msg: Option<&dyn IPacketMsg>,
name: &str)` is the C# `IPacketMsgExtensions.GetMsgTypeWithNullCheck`: Rust has no null
references, so the null check is structural, and the function survives as the extension
point it was.
**Notes and gotchas:** the C# `IPacketMsg` exposes `ulong TargetJobID` /
`SourceJobID`; the Rust wraps them in `JobID` and uses `JobID::from_value(0)` as the
"absent" default, matching the C# `ulong` default of 0.
stores `Header` and `BodyOffset` in `internal` fields; the Rust keeps them `pub(crate)`
and exposes `header()` / `body_offset()` publicly. The C# `GetData()` returns the
original `byte[]`; the Rust returns a borrow of the owned `Vec<u8>`. The C# deserialises
the header from a `MemoryStream` over the whole frame; the Rust uses a `Reader` over the
same bytes and takes `reader.position()` as the body offset. The C# extension class
became two free functions (`get_msg_type_with_null_check` here and in
`base/gc/packet_base_gc.rs`) because Rust has no extension methods.

#### `crates/steamkit/src/base/gc/msg_base_gc.rs` — 50 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/GC/MsgBaseGC.cs`
**Purpose:** the game-coordinator counterparts of `IClientMsg` and of the generic message
base.
**How it works:** `trait IClientGCMsg: Send + Sync` declares `is_proto()`, `msg_type() ->
u32`, `target_job_id()` / `set_target_job_id()`, `source_job_id()` /
`set_source_job_id()`, `serialize() -> Vec<u8>` and `deserialize(&mut self, data: &[u8])
-> Result<(), SteamError>`. The C# `SteamKit2.GC.IClientGCMsg` is the same set; its
`Deserialize(byte[] data)` returns `void` and throws, the Rust returns a `Result`.
`GCMsgBase<THeader> { header: THeader, payload: MsgBase }` is the Rust counterpart of the
C# `public abstract class GCMsgBase<THeader> : MsgBase, IClientGCMsg`: the C# class
inherits the `Payload` stream from `MsgBase` and declares `IsProto`, `MsgType` and the
job ids abstract, while the Rust struct holds the header and the payload explicitly.
**Notes and gotchas:** **`GCMsgBase` is currently unused** — a grep over the Rust tree
finds no construction site, because the concrete `ClientGCMsgProtobuf<TBody>` and
`ClientGCMsg<TBody>` in `base/gc/client_msg_gc.rs` each carry their own `header` and
`payload` fields instead of embedding a `GCMsgBase`. It is kept as the public
counterpart of the C# class. The C# constrains the header with `where THeader :
IGCSerializableHeader, new()`; the Rust `GCMsgBase<THeader>` has **no** constraint, and
`new(header, payload_reserve)` takes the header by value. The `MsgType` of a
game-coordinator message is a plain `uint`, not an `EMsg` — the GC protocol has its own
message numbering. The job-id fields in the GC protobuf header are named
`job_id_target` / `job_id_source` (the Steam ones are `jobid_target` / `jobid_source`) —
see `steamkit_protos::gc::CMsgProtoBufHeader`.

#### `crates/steamkit/src/base/gc/client_msg_gc.rs` — 299 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/GC/ClientMsgGC.cs`
**Purpose:** the game-coordinator message envelopes — `ClientGCMsgProtobuf<TBody>` and
`ClientGCMsg<TBody>`.
**How it works:** `ClientGCMsgProtobuf<TBody: prost::Message + Default>` holds
`header: MsgGCHdrProtoBuf`, `body: TBody` and `payload: MsgBase`. `new(e_msg: u32)` /
`with_payload_reserve(e_msg, reserve)` build it; `reply(e_msg, reply_to)` sets the target
job id to the source job id of the message being answered (the C#
`Header.Proto.job_id_target = msg.Header.Proto.job_id_source`); `from_packet(&dyn
IPacketGCMsg)` errors for a non-proto packet; `serialize()` writes the header, the body
and the payload; `deserialize(&[u8])` reads the `MsgGCHdrProtoBuf`, decodes the body from
the remainder and stores the leftover as the payload. `ClientGCMsg<TBody>` holds
`header: MsgGCHdr`, `body: TBody`, `payload: MsgBase` and `msg_type: u32`; `new(
payload_reserve)` takes the message type from `body.get_gc_emsg()`, exactly like the C#
`msgType = Body.GetEMsg()`; `reply(reply_to)` sets the target job id; `from_packet`
errors for a *proto* packet.
**Notes and gotchas:** the C# `ClientGCMsgProtobuf` has **no** header-only non-generic
variant — unlike `ClientMsgProtobuf` there is only the generic class — and the Rust
matches: there is exactly one `ClientGCMsgProtobuf`. The C# checks the packet kind with
`DebugLog.Assert(msg.IsProto, ...)`, an assertion rather than an exception; the Rust
turns it into a hard `SteamError::InvalidArgument("ClientGCMsgProtobuf used for
non-proto message!")` / `"ClientGCMsg used for proto message!"`, so the Rust is
stricter. The C# `Deserialize` writes the trailing bytes into `Payload` **without**
seeking the payload back to 0 and without clearing it first, so a second `Deserialize` on
the same instance appends; the Rust replaces `self.payload` wholesale and seeks to 0.
The C# `ClientGCMsgProtobuf.Deserialize` reads the header and then
`Serializer.Deserialize<TBody>(ms)` — protobuf-net reads until the end of the stream, so
the payload is normally empty — and the Rust does the same with `prost`. The
game-coordinator protobuf header is `steamkit_protos::gc::CMsgProtoBufHeader`, a
*different* message from the Steam one. `ClientGCMsg<TBody>` keeps `msg_type` in a
separate field (the C# keeps it in a `readonly uint msgType`), whereas
`ClientGCMsgProtobuf<TBody>` takes it from `header.msg`.

#### `crates/steamkit/src/base/gc/packet_base_gc.rs` — 144 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/GC/PacketBaseGC.cs`
**Purpose:** the immutable game-coordinator view of an inbound frame.
**How it works:** `trait IPacketGCMsg: Send + Sync` declares `is_proto()`, `msg_type() ->
u32`, `target_job_id()`, `source_job_id()` and `get_data() -> &[u8]`.
`PacketClientGCMsgProtobuf` parses the `MsgGCHdrProtoBuf` to obtain `job_id_target` and
`job_id_source` and caches them as `JobID`s; `PacketClientGCMsg` parses the `MsgGCHdr`
and caches its two job ids. `get_msg_type_with_null_check(msg: Option<&dyn IPacketGCMsg>,
name: &str)` is the C# `IPacketGCMsgExtensions.GetMsgTypeWithNullCheck`.
**Notes and gotchas:** the C# `PacketClientGCMsgProtobuf` and `PacketClientGCMsg` have
**no body offset** — the GC packet types expose only the header fields and the raw data,
and the body offset is recomputed by `ClientGCMsg*::deserialize`. The Rust is the same:
there is no `body_offset()` on the GC packet types, unlike `base/packet_base.rs`. The C#
`IPacketGCMsg` already exposes `JobID TargetJobID` (unlike the Steam `IPacketMsg`, which
exposes `ulong`), and the Rust returns `JobID` on both sides, so the GC path is
type-safe end to end. The C# stores the job ids as `readonly` fields assigned in the
constructor; the Rust does the same.

#### `crates/steamkit/src/base/generated/steam_language_internal.rs` — 1168 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/Generated/SteamLanguageInternal.cs`
**Purpose:** the hand-written half of the Steam wire format — the serialisation traits
and the concrete message and header structures with their exact byte layouts. A wrong
field width here would silently corrupt every message, so this layer was fixed by hand
and is verified mechanically.
**How it works:** the traits are `SteamSerializable` (`fn write(&self, out: &mut
Vec<u8>)`), `SteamSerializableDeserialize: SteamSerializable + Default + Sized` (`fn
read(r: &mut Reader<'_>) -> Result<Self, SteamError>` — the `Default` bound replaces the
C# `new()` constraint), `ISteamSerializable`, `ISteamSerializableHeader` (`set_emsg`),
`ISteamSerializableMessage` (`get_emsg`), `IGCSerializableHeader` (`set_gc_emsg`) and
`IGCSerializableMessage` (`get_gc_emsg`). The C# interface names are re-exported under
Rust-friendly aliases (`SteamSerializableHeader`, `SteamSerializableMessage`,
`GcSerializableHeader`, `GcSerializableMessage`).

`WireValue` is `fn write_wire(&self, out: &mut Vec<u8>)` and `fn read_wire(r: &mut
Reader<'_>) -> Result<Self, SteamError>`, implemented for every primitive (`u8 i8 u16 i16
u32 i32 u64 i64 f32 f64 bool`) and for every Valve enum that travels on the wire. The
wire behaviour lives in the *type*, not in the macro: a `macro_rules!` fragment captured
as `$kind:ty` is opaque and cannot be re-matched against literal tokens, so a
token-matching macro cannot tell a `u32` field from an enum field. This was the
resolution of defect #5 in the test-suite bug table in §12 of this guide. `WireEnum` adds
`wire_value() -> i32`, `from_wire_value(i32) -> Option<Self>` and `wire_size() -> usize`
(default 4). `EUdpPacketType` overrides `wire_size()` to **1** — the C# writes
`bw.Write((byte)PacketType)` for the UDP packet type while every other enum is written
as `bw.Write((int)Enum)` — and implements `from_wire_value` by narrowing to `u8` first,
so a value above 255 is rejected rather than truncated. `read_bool` reads one byte and
returns `byte == 1`, because the C# structs store their booleans in a private `byte`
with a `== 1` getter.

`steam_serializable!` implements `SteamSerializable`/`SteamSerializableDeserialize` for a
structure, writing and reading each field through `WireValue` in declaration order:
```rust
steam_serializable!(generated::MsgHdr {
    msg: EMsg = EMsg::Invalid,
    target_job_id: u64 = u64::MAX,
    source_job_id: u64 = u64::MAX,
});
```
`steam_message!` implements `ISteamSerializableMessage` with a fixed `EMsg`;
`steam_header!` implements `ISteamSerializableHeader` by writing `self.msg`.

`MsgHdrProtoBuf` is the protobuf client message header. Its wire layout is: the
protobuf-flagged message type (4 bytes LE), the length of the serialised protobuf header
(4 bytes LE), then that many bytes of `CMsgProtoBufHeader`. `write` uses
`make_msg(self.msg, true)` — the message type **with** the `0x8000_0000` protobuf mask —
then the encoded protobuf header; `read` reads the raw `i32`, strips the mask with
`get_msg`, reads the header length (rejecting a negative length) and decodes the
protobuf header. `MsgGCHdrProtoBuf` is the game-coordinator equivalent, using
`steamkit_protos::gc::CMsgProtoBufHeader` and `make_gc_msg`. The mask itself is
`PROTO_MASK = 0x8000_0000`, the C# `MsgUtil.ProtoMask`; `make_msg(msg, protobuf)` sets
the bit, `get_msg(raw)` clears it and maps the value back to an `EMsg`, and
`make_gc_msg` / `get_gc_msg` are the game-coordinator equivalents.

The structures are `UdpHeader` (36 bytes), `ChallengeData` (8), `ConnectData` (4),
`Accept` / `Datagram` / `Disconnect` (0), `MsgHdr` (20: `EMsg` plus two `u64`),
`ExtendedClientMsgHdr` (36: `EMsg`, `u8` header size 36, `u16` header version 2, two
`u64` job ids, `u8` canary 239, `u64` SteamID, `i32` session id), `MsgGCHdr` (18:
`u16` header version 1 plus two `u64`) and about forty message bodies
(`MsgClientLogon`, `MsgClientChatEnter`, `MsgClientP2PIntroducerMessage`, …).
`MsgClientP2PIntroducerMessage` is written and read by hand because of its fixed-size
buffer: the C# declares `byte[] Data = new byte[1450]`, so exactly 1450 bytes always
travel on the wire regardless of what `DataLen` announces. The Rust writer pads or
truncates `data` to exactly `P2P_INTRODUCER_DATA_SIZE = 1450` and the reader always
reads 1450 bytes, giving a total message size of 8 + 4 + 1450 + 4 = 1466.

The plain data structures live in `crate::base::generated::steam_language::internal`
(generated from the `.steamd` files) and are **re-exported** from this file, so every C#
type of `SteamLanguageInternal.cs` is reachable through
`steamkit::base::generated::steam_language_internal`.

**Notes and gotchas:** ***`EUdpPacketType` is one byte; every other enum is four.*** The
port had it wrong at first; the width is carried by `WireEnum::wire_size()`, and
`tools/verify_wire_format.py` compares the C# `bw.Write(...)` widths per struct against
the Rust field kinds: **145 fields compared, 0 mismatches**. *`get_msg` returns
`Option<EMsg>`* and `MsgHdrProtoBuf::read` maps an unknown message number to
`EMsg::Invalid`; the C# casts the raw value to `EMsg` unconditionally, producing an
undefined enum value. The Rust is more forgiving, but code that switches on
`EMsg::Invalid` will treat an unknown message as `Invalid`. *`read_bool` returns `false`
for any byte other than 1* — including 0 **and** 2. That is exactly the C# behaviour
(the field is a `byte` and the getter is `== 1`), but it is surprising. `steam_serializable!`
writes the fields in declaration order with no padding; the C# classes use
`[StructLayout(LayoutKind.Sequential)]` and write each field explicitly in `Serialize`,
and the two agree because the generator's field order mirrors the `.steamd` declaration
order. The defaults in the macro are the C# field initialisers (`header_size = 36`,
`header_version = 2`, `header_canary = 239`, `source_conn_id = 512`, …).
`EUdpPacketType::from_wire_value` narrows to `u8` *before* the lookup, so a wire value
of 256 is an error rather than a truncation to 0. *There are two `MsgHdrProtoBuf` types.*
The hand-written one at the top of this file carries the **typed** `CMsgProtoBufHeader`
and is what `base/client_msg.rs` and `base/packet_base.rs` use; the generated
`internal::MsgHdrProtoBuf` carries the **raw** protobuf bytes (`proto: Vec<u8>`) and
faithfully mirrors the C# `MsgHdrProtoBuf` in `SteamLanguageInternal.cs`. Use the
hand-written one unless you specifically need the raw byte form. The file ends with a
`#[cfg(test)] mod temp_wire_verification` that asserts the exact serialised length of
every structure (`UdpHeader` = 36, `MsgHdr` = 20, `ExtendedClientMsgHdr` = 36,
`MsgGCHdr` = 18, `MsgClientCreateChat` = 53, `MsgClientP2PIntroducerMessage` = 1466, …)
and checks the byte-exact `UdpHeader` layout.

#### `crates/steamkit/src/base/generated/steam_language.rs` — 18785 lines
**C# counterpart:** `SteamKit2/SteamKit2/Base/Generated/SteamLanguage.cs` (the 91
Valve-internal enums); for the `internal` module, the message and header classes of
`SteamKit2/SteamKit2/Base/Generated/SteamLanguageInternal.cs`
**Purpose:** the machine-generated Rust translation of Valve's SteamLanguage definition
files. It carries the 91 Valve-internal enumerations (`EMsg`, `EResult`, `EUniverse`,
`EAccountType`, `EDepotFileFlag`, `EUdpPacketType`, …) and, in the nested `internal`
module, the plain data structures declared by the same definition files (`UdpHeader`,
`MsgHdr`, `ExtendedClientMsgHdr`, `MsgGCHdr`, the message bodies). It is the single
source of truth for every message number and every flag bit in the port.
**How it works:** *generation* — the file is produced by `steamkit-lang-gen`, the Rust
port of the C# `Resources/SteamLanguageParser`:
```text
cargo run -p steamkit-tools --bin steamkit-lang-gen
```
The inputs are the Valve definition files in `crates/steamkit-tools/lang/`
(`emsg.steamd`, `eresult.steamd`, `enums.steamd`, `gamecoordinator.steamd`,
`header.steamd`, `netheader.steamd`, `steammsg.steamd`), which use `#import` to pull each
other in; the default entry file is `steammsg.steamd` and the default output is
`crates/steamkit/src/base/generated/steam_language.rs`. The tool also accepts
`--lang-dir`, `--entry`, `--out` and `--stdout`. The generator lives in
`crates/steamkit-tools/src/generator.rs` (the emitter, a port of
`Resources/SteamLanguageParser/Generator/CSharpGen.cs`), with `src/tokenizer.rs` (the
lexer), `src/analyzer.rs` (the parser) and `src/main.rs` (the binary). The output is
verified against the C# by `tools/verify_steam_language.py`: **91 enums, 3093 variants,
0 mismatches**.

*naming* — enum variants keep the Valve name verbatim (`EMsg::ClientLogon`), because a
rename would break every call site that mirrors the C#. Struct **fields** are converted
to `snake_case` (`targetJobID` → `target_job_id`) and class constants to
`SCREAMING_SNAKE_CASE`; a name that collides with a Rust keyword becomes a raw
identifier (`r#type`, the `type` field of `MsgClientChatMemberInfo`). The file carries
`#![allow(non_camel_case_types)]` because the Valve names are PascalCase.

*semantics* — the generator reproduces the C# compiler's value semantics exactly: a
member marked `removed` is **not emitted** at all (and does not take part in the
auto-increment); a member marked `obsolete` is emitted and marked `#[deprecated]` (with
the reason string, when the source has one); a member without an explicit value
continues from the previous member (`previous + 1`), starting at 0; a `|`-separated
value is the bitwise or of its terms, where a term is either a literal or a previously
declared member of the same enum (or a literal member of another enum); the underlying
type is the declared one (`byte`, `ushort`, `uint`, `long`, `ulong`, …) or `int` by
default, and the Rust enum gets the matching `#[repr(...)]`.

*duplicate values → associated consts* — C# permits two enum members to share a value
(`EMsg.BaseGeneral = 100` and `EMsg.GenericReply = 100`); Rust forbids duplicate
discriminants (`E0081`). The generator therefore emits the **first** member with a value
as a real variant and every later member with the same value as an associated constant:
```rust
/// Alias of [`EMsg::BaseGeneral`].
pub const GenericReply: EMsg = EMsg::BaseGeneral;
```
There are 39 such aliases across the 91 enums (3054 real variants + 39 aliases = the
C#'s 3093 members). `EMsg::GenericReply == EMsg::BaseGeneral`, and `EMsg::from_value(100)`
returns `BaseGeneral` — exactly how the C# resolves the cast.

*`value()` / `from_value()`* — every enum gets `pub const fn value(self) -> i32` (or the
underlying type), which is `self as <underlying>`; `pub fn from_value(value: i32) ->
Option<Self>`, a `match` over the numeric values that returns the **first declared**
variant with that value — i.e. the same variant a C# cast would produce — and `None` for
an unknown value; `pub const ALL: &'static [Self]`, every variant **and** every alias in
declaration order; `impl TryFrom<i32>` (plus `TryFrom<u32>` when every value fits) with
`Error = UnknownEnumValue`; and `impl From<Self> for <underlying>`. Flag enums
additionally get `bits()`, `is_empty()`, `contains(other)`, `union(other)` and
`intersect(other)`.

*`UnknownEnumValue`* — a small error type carrying the enum name and the offending
value, with `Display`, `std::error::Error` and `From<UnknownEnumValue> for SteamError`
(which converts it to `SteamError::InvalidArgument`).

*the `internal` module* — the message and header structures are emitted into `pub mod
internal { … }` so the enums stay at the root of `steam_language`. Each structure gets
its fields, a `Default` impl carrying the C# field initialisers, and the class constants
as associated `const`s (for example `UdpHeader::MAGIC = 825250646`).

**Notes and gotchas:** *never edit this file by hand* — it is regenerated, and every
manual edit is lost. Change the `.steamd` sources or the generator and re-run the
binary. *regenerating and verifying*:
```text
cargo run -p steamkit-tools --bin steamkit-lang-gen   # regenerate
python tools/verify_steam_language.py                  # compare against the C#
python tools/verify_wire_format.py                     # compare the field widths
```
`verify_steam_language.py` resolves the associated-const aliases before comparing, so the
alias mechanism does not weaken the check. *the `internal` module holds data only* — the
serialisation of those structures lives in
`base/generated/steam_language_internal.rs`, which implements the `WireValue` /
`SteamSerializable` traits for them. The generated file cannot decide a field's wire
width; that is precisely why the `WireValue` trait exists. *aliases are constants, not
variants* — `EMsg::GenericReply` is not a variant: it cannot be matched as a pattern the
way `EMsg::BaseGeneral` can, and it does not participate in an exhaustive `match`. Use
the primary variant in patterns. *`from_value` returns the first declared variant* — for
`EMsg`, `from_value(100)` is `BaseGeneral`, never `GenericReply`; if you need to know
which name Valve gave a number, `from_value` will not tell you. *`ALL` contains the
aliases too* — `EMsg::ALL` has 3093 entries, not 3054, because it is the C#'s member
list; `ALL.len()` is therefore not the number of distinct values. *`removed` members are
gone* — a member the `.steamd` marks `removed` does not exist in the Rust enum at all,
so a ported call site that referenced it has to be rewritten rather than renamed.
*`#[deprecated]` members still exist* — `EMsg::AMCreateAccountResponse` and friends carry
`#[deprecated]`; using one is a warning rather than an error, and the associated-const
block is `#[allow(deprecated)]` so the aliases can point at deprecated variants.

### 5.3 `crates/steamkit/src/util/` — the helpers, and the vendored LZMA SDK


`util/` is the Rust port of `SteamKit2/SteamKit2/Util`. It contains 26 top-level modules plus a
vendored subtree at `util/lzma/`, and it falls into five families:

| Family | Modules |
|---|---|
| Crypto & checksums | `crypto_helper.rs`, `adler32.rs` |
| Logging & diagnostics | `debug_log.rs`, `i_log_context.rs`, `simple_console_debug_listener.rs`, `debug_network_listener.rs` |
| Platform / OS plumbing | `hardware_utils.rs`, `mac_helpers.rs`, `win32_helpers.rs`, `utils.rs`, `net_helpers.rs`, `date_utils.rs`, `value_stopwatch.rs`, `scheduled_function.rs`, `task_extensions.rs`, `stream_helpers.rs` |
| Steam wire formats | `msg_util.rs`, `key_dictionary.rs`, `steam_kit_web_request_exception.rs` |
| Container formats | `lzma_decoder.rs`, `lzma_util.rs`, `zip_util.rs`, `v_zip_util.rs`, `v_zstd_util.rs`, `v_zip_delta_util.rs`, and the vendored `lzma/` SDK tree (compress side only) |

#### Conventions used throughout `util/`

* **Static classes become unit structs.** Each C# `static class X` becomes `pub struct X;` with
  associated functions (`MsgUtil::get_msg`, `DateUtils::date_time_from_unix_time`, …). Because Rust
  has no namespace-level functions, every module also exports free-function aliases that simply
  forward to the associated function, so call sites can read either way. Where the C# class was
  `internal static`, the Rust struct is `pub(crate)`.
* **`Task` / `CancellationToken` become tokio.** `ScheduledFunction` is a `tokio::time::interval`
  inside a spawned task that is cancelled through a `CancellationToken`; `TaskExtensions`
  becomes a `Future` adapter (`IgnoringCancellation<F>`) instead of a `Task` extension method.
* **`#![forbid(unsafe_code)]`.** The two places that genuinely need FFI — Windows
  `CreateFile`/`DeviceIoControl` and macOS IOKit/CoreFoundation/DiskArbitration — are
  reimplemented on top of the command line tools the platforms already ship
  (`powershell`/`pwsh`, `ioreg`, `diskutil`, `reg`, `cmd /c ver`). See
  `win32_helpers.rs` and `mac_helpers.rs`.
* **Exceptions become `SteamError`.** `ArgumentException` → `SteamError::InvalidArgument`,
  `InvalidDataException` → `SteamError::Decompression`, `DirectoryNotFoundException` →
  `SteamError::NotFound`, and so on. The C# `DebugLog.Assert` calls are kept as
  `DebugLog::assert(...)`, which logs but does not abort.
* **No `chrono`.** `date_utils.rs` hand-rolls the epoch arithmetic on `std::time::SystemTime`.
* **Line counts.** The counts in the section headers are non-blank lines
  (`Get-Content | Measure-Object -Line`), matching the convention used elsewhere in this guide.

Three modules in `util/` are currently **not called from anywhere outside `util/`**:
`scheduled_function.rs`, `value_stopwatch.rs`, `task_extensions.rs`, `stream_helpers.rs`,
`lzma_util.rs` and `v_zip_delta_util.rs`. They are complete ports of the corresponding C# APIs
and are kept so that the public surface matches the C#; the consumers live in code that has not
been written yet (the CM protocol reader, the HTTP layer) or in the tools.

---

#### `util/lzma/` — the vendored 7-Zip LZMA SDK (compress side only)

`util/lzma/` is a port of the 7-Zip LZMA SDK that SteamKit2 vendors under
`SteamKit2/Util/LZMA/`. The Rust tree mirrors the C# directory layout one-for-one:

```
util/lzma/i_coder.rs                          <->  Util/LZMA/ICoder.cs
util/lzma/common/crc.rs                       <->  Util/LZMA/Common/CRC.cs
util/lzma/compress/lz/i_match_finder.rs       <->  Util/LZMA/Compress/LZ/IMatchFinder.cs
util/lzma/compress/lz/lz_bin_tree.rs          <->  Util/LZMA/Compress/LZ/LzBinTree.cs
util/lzma/compress/lz/lz_in_window.rs         <->  Util/LZMA/Compress/LZ/LzInWindow.cs
util/lzma/compress/lzma/lzma_base.rs          <->  Util/LZMA/Compress/LZMA/LzmaBase.cs
util/lzma/compress/lzma/lzma_encoder.rs       <->  Util/LZMA/Compress/LZMA/LzmaEncoder.cs
util/lzma/compress/range_coder/range_coder.rs          <->  Util/LZMA/Compress/RangeCoder/RangeCoder.cs
util/lzma/compress/range_coder/range_coder_bit.rs      <->  Util/LZMA/Compress/RangeCoder/RangeCoderBit.cs
util/lzma/compress/range_coder/range_coder_bit_tree.rs <->  Util/LZMA/Compress/RangeCoder/RangeCoderBitTree.cs
```

The C# namespaces (`SevenZip`, `SevenZip.Compression.LZ`, `SevenZip.Compression.LZMA`,
`SevenZip.Compression.RangeCoder`) became module paths, and the SDK's `Stream` parameters became
owned `Vec<u8>` buffers — the encoder is fully in-memory in both ports.

#### The split: `util/lzma/` is the *compress* side only

This is the single most confusing thing about the layout, and it is inherited verbatim from the
C# tree:

* The **LZMA SDK is vendored compress-side only.** `Util/LZMA/` contains `Compress/` and
  `Common/` but no decoder — the SDK's `Decoder.cs` / `RangeCoder` decoder were never copied in.
* The **decoder is SteamKit's own**, hand-written, in `Util/LzmaDecoder.cs`.
* Therefore in Rust: **`util/lzma/` is the encoder, `util/lzma_decoder.rs` is the decoder**, and
  they are two completely independent implementations of the LZMA bitstream format that happen
  to interoperate. They share no code; they agree only on the wire format.

Concretely:

* `util/lzma_decoder.rs` implements the **range *decoder*** (arithmetic decoding with adaptive
  probabilities, `decode_bit` / `decode_tree_bit` / `decode_literal` / `decode_matched_literal` /
  `decode_length` / `decode_distance`). It is what Steam actually uses to read data.
* `util/lzma/compress/range_coder/` implements the **range *encoder*** (`Encoder::encode`,
  `BitEncoder`, `BitTreeEncoder`) and the **optimal-parsing parser** that decides which literals
  and matches to emit. It exists only because `VZipUtil::compress` has to produce a VZip
  container, and a VZip container holds an LZMA stream.

If you are looking for "how do we *read* LZMA", the answer is `util/lzma_decoder.rs`. If you are
looking for "how do we *write* LZMA", the answer is `util/lzma/compress/`.

---

#### File reference

#### `crates/steamkit/src/util/adler32.rs` — 51 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/Adler32.cs`
**Purpose:** Computes the Adler-32 checksum (RFC 1950) of a byte buffer, optionally continuing from
a previously returned running value. Steam uses it as the integrity checksum of a decompressed
depot chunk.

**How it works:** `Adler32::calculate(adler: u32, buffer: &[u8]) -> u32` splits the incoming
running value into `s1 = adler & 0xFFFF` and `s2 = (adler >> 16) & 0xFFFF`, folds every byte with
`s1 = s1 + byte; s2 = s2 + s1`, and returns `(s2 << 16) | s1`. The sums are reduced modulo
`BASE = 65521` after every chunk of at most `NMAX = 5552` bytes — the largest `n` for which
`255n(n+1)/2 + (n+1)(BASE-1) <= 2^32-1` — so that they cannot overflow. The inner loop is
unrolled 16 bytes at a time. All arithmetic uses `wrapping_add`, mirroring the C#'s unchecked
`uint` arithmetic.

**Notes and gotchas:**
* The C# class has **three** implementations of the same algorithm — `CalculateScalar`,
  `CalculateVector128` (`Vector128<T>`) and `CalculateVector256` (`Vector256<T>`) — selected at
  runtime by CPU feature detection. All three produce identical digests, so **only the scalar path
  was ported**. The Rust implementation is the reference RFC 1950 path and is used for every input
  size; there is no SIMD path and no feature-detection branch.
* The C# `BlockSize` is 32 and it reduces the sums every `NMAX / BlockSize = 173` blocks; the Rust
  reduces every 5552 bytes and unrolls by 16. Same result, different blocking.
* **The crate does not actually use this module.** `steam/cdn/depot_chunk.rs` has its own inline
  `DepotChunk::adler_hash`, which is the counterpart of the C# `Adler32.Calculate(0, input)`.
  That copy seeds `s1` with **zero** rather than with the RFC's usual 1 — the C# does the same,
  because `Adler32.Calculate` seeds `s1` from the low 16 bits of the supplied seed, and the seed
  is `0`. Do not "fix" either implementation to use the RFC's initial `1`; it would break chunk
  validation.
* `util/adler32.rs` is nevertheless the faithful port and is the module to use for a general
  Adler-32; the duplication in `depot_chunk.rs` is a known wart.

#### `crates/steamkit/src/util/crypto_helper.rs` — 88 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/CryptoHelper.cs`
**Purpose:** Decrypts a blob with the Steam symmetric scheme: AES-256/CBC/PKCS7, with the random
IV prepended in AES-256/ECB form. This is the counterpart of the C# `CryptoHelper.SymmetricDecrypt`.

**How it works:** `CryptoHelper::symmetric_decrypt(input: &[u8], key: &[u8]) -> Result<Vec<u8>,
SteamError>` performs three steps.

1. **Key check.** The key must be exactly 32 bytes (`KEY_SIZE`). Otherwise the C#
   `DebugLog.Assert(key.Length == 32, …)` is mirrored by `DebugLog::assert(false, "CryptoHelper",
   "SymmetricDecrypt used with non 32 byte key!")` and the function returns
   `SteamError::InvalidArgument`.
2. **IV recovery.** The first 16 bytes (`BLOCK_SIZE`) of `input` are the ECB-encrypted IV. They
   are decrypted with `ecb::Decryptor::<aes::Aes256>` in place (`decrypt_block_mut`) into a
   16-byte `iv` array. An input shorter than 16 bytes is rejected with
   `SteamError::Crypto`.
3. **CBC decryption.** The remainder (`input[16..]`) is decrypted with
   `cbc::Decryptor::<aes::Aes256>::new_from_slices(key, &iv)` and
   `decrypt_padded_mut::<Pkcs7>`, which strips the PKCS#7 padding and returns the plaintext
   slice. The ciphertext length must be non-zero and a multiple of 16, otherwise
   `SteamError::Crypto`.

The module also exports a free function `symmetric_decrypt(input, key)` that forwards to the
associated function.

**Notes and gotchas:**
* **There is no HMAC in this module.** The C# `CryptoHelper` has exactly one method,
  `SymmetricDecrypt`; it has no HMAC helper. Steam's HMAC-SHA1 channel MAC lives in a different
  file in both trees — in Rust it is `networking/steam3/net_filter_encryption_with_hmac.rs`
  (C# `Networking/Steam3/NetFilterEncryptionWithHMAC.cs`), not here. If you were looking for
  "the AES and HMAC helpers" in `util/crypto_helper.rs`, the HMAC half is not in this file.
* **Port bug caught by the test suite (bug #1 in §12 of this guide).** The
  first version of the port allocated a **zeroed** buffer
  (`let mut buffer = vec![0u8; ciphertext.len();]`) and handed it straight to
  `decrypt_padded_mut`, **never copying the ciphertext into it**. RustCrypto's
  `decrypt_padded_mut` decrypts the buffer *in place*, so the function silently decrypted a block
  of zeroes and returned garbage — it had never decrypted anything at all. The fix is the
  `buffer.copy_from_slice(ciphertext);` on the line before the decryptor is built. The same
  zeroed-buffer bug appeared a second time in `steam/cdn/depot_chunk.rs` (bug #2), where the IV
  was additionally being decrypted from the zeroed `iv` array instead of from `data[..16]`.
* The C# `Aes.DecryptEbc(input[..16], iv, PaddingMode.None)` and
  `Aes.DecryptCbc(input[16..], iv, PaddingMode.PKCS7)` map onto the RustCrypto `ecb`/`cbc`
  crates with `aes` as the block cipher. Padding validation differs in *kind*, not in *result*:
  the C# throws `CryptographicException` on bad padding, the Rust returns
  `SteamError::Crypto("Failed to decrypt the payload.")`.
* The C# `ArgumentNullException.ThrowIfNull(key)` has no Rust analogue — a `None` key is simply
  not expressible; a short key is.
* The key length is validated with `DebugLog::assert` *and* an error return. `DebugLog::assert`
  only logs, so the error return is the part that actually enforces the contract.
* The same "ECB-encrypted IV prefix + CBC/PKCS7 body" layout is used for depot chunks; the
  chunk code lives in `steam/cdn/depot_chunk.rs` and duplicates this logic rather than calling
  this module.

#### `crates/steamkit/src/util/date_utils.rs` — 50 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/DateUtils.cs`
**Purpose:** Converts between Unix timestamps and points in time, for the protobuf fields Steam
sends as Unix seconds (`CMsgClientLogon`, app info timestamps, job completion times, …).

**How it works:** `DateUtils::date_time_from_unix_time(unix_time: u64) -> Result<SystemTime,
SteamError>` returns `UNIX_EPOCH + Duration::from_secs(unix_time)`, rejecting timestamps above
`MAX_UNIX_SECONDS = 253_402_300_799` (9999-12-31T23:59:59Z, the largest value .NET's `DateTime`
can hold). `DateUtils::date_time_to_unix_time(time: SystemTime) -> Result<u64, SteamError>`
returns `time.duration_since(UNIX_EPOCH)?.as_secs()`. Both are re-exported as free functions.

**Notes and gotchas:**
* The C# returns `DateTime` (an *unspecified-kind* local time in `DateTimeFromUnixTime`) and
  `ulong`. The Rust port uses `SystemTime`, which is always UTC, and `u64`. There is no timezone
  conversion anywhere in the port — `DateTimeOffset.FromUnixTimeSeconds(..).UtcDateTime` in the
  C# is likewise a UTC instant.
* **The Rust port rejects pre-epoch instants; the C# silently produces a huge `ulong`.** The C#
  `DateTimeToUnixTime` computes `(long)(time - UnixEpoch).TotalSeconds` and casts to `ulong`, so a
  date before 1970 becomes a value near `2^64`. The Rust version returns
  `SteamError::InvalidArgument("The date is before the unix epoch (…)")` instead. Any code that
  relied on the C#'s wraparound would behave differently — nothing in the crate does.
* **Sub-second precision is truncated towards the epoch** (`as_secs()`), exactly like .NET's
  `DateTimeOffset.ToUnixTimeSeconds`, which truncates rather than rounds.
* The Rust crate has **no `chrono` dependency** (the C# has no such concept). the porting notes
  records that `chrono` and `percent-encoding` were originally used but never declared in
  `Cargo.toml` and were later removed in favour of hand-rolled epoch arithmetic.
* `DateUtils` is used from `types/manifest.rs`, `steam/handlers/steam_apps/callbacks.rs`,
  `steam/handlers/steam_cloud/callbacks.rs`, `steam/handlers/steam_content/steam_content.rs`
  and `steam/handlers/steam_friends/callbacks.rs`.
* `debug_network_listener.rs` uses `DateUtils::date_time_to_unix_time` to name its capture
  directory; it swallows the error with `.unwrap_or_default()`, so a pre-epoch clock would name
  the directory `0`.

#### `crates/steamkit/src/util/debug_log.rs` — 255 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/DebugLog.cs`
**Purpose:** The global debug log: a registry of listeners that every part of the library writes
to, plus the composite-format helper used to build the messages.

**How it works:** All state lives in a single `static STATE: OnceLock<DebugLogState>`. The state
holds

* `enabled: AtomicBool` — initialised to `cfg!(debug_assertions)`, matching the C# static
  constructor (`#if DEBUG Enabled = true; #else Enabled = false;`),
* `listeners: RwLock<Vec<Arc<dyn IDebugListener>>>` — the object listeners,
* `functions: RwLock<Vec<(u64, Arc<dyn Fn(&str, &str) + Send + Sync>)>>` — the closure listeners,
  each with a monotonically increasing `u64` handle from `next_function_id: AtomicU64`,
* `sender: broadcast::Sender<DebugLogMessage>` — a tokio broadcast channel with
  `BROADCAST_CAPACITY = 1024`.

`DebugLog::write_line(category, msg)` returns immediately when logging is disabled, otherwise it
publishes a `DebugLogMessage { category, message }` on the broadcast channel and then invokes
every object listener and every closure listener. `DebugLog::add_listener` /
`remove_listener` / `clear_listeners` manage the lists; `remove_listener` compares by
**identity** (`Arc::ptr_eq`), which is the Rust equivalent of the C#'s
`List<IDebugListener>.Remove(listener)` reference-equality removal.
`DebugLog::add_listener_function` returns a `u64` handle that `remove_listener_function` takes,
because Rust closures cannot be compared for equality the way C# delegates can.
`DebugLog::subscribe()` hands out a `broadcast::Receiver<DebugLogMessage>` for consumers that
prefer a channel over a callback — this is an addition with no C# counterpart.
`DebugLog::assert(condition, category, message)` logs `"Assertion Failed! {message}"` when the
condition is false; the C# additionally calls `System.Diagnostics.Debug.Assert`, which is a no-op
in release builds, so the observable behaviour is the same.

**Notes and gotchas:**
* The trait is named `IDebugListener` and the listener callback is `write_line(&self, category,
  msg)`. The C# interface is `IDebugListener.WriteLine(string category, string msg)`.
* **The C# `AddListener(Action<string,string>)` overload is split in two.** In C# the action
  overload wraps the closure in a private `ActionListener` class and lives in the same list, so
  `RemoveListener(action)` finds it by delegate equality. The Rust port keeps the closures in a
  **separate** list with an explicit `u64` handle. `remove_listener` therefore only ever removes
  object listeners; `remove_listener_function` only ever removes closures.
* **`format_with_args` implements only a subset of .NET composite formatting.** It handles
  positional placeholders `{0}`, `{1}`, … and the `{{` / `}}` escapes. A placeholder that carries
  a format specifier or an alignment (`{0:X}`, `{0,10}`) is **copied verbatim**, because the Rust
  `args` are `&[&str]` and there is no way to apply .NET's format specifiers to a string. The C#
  passes `object?[]` and uses `string.Format`, so `{0:X}` on an `int` really formats as hex. Any
  C# call site that relied on a format specifier must be rewritten to format the value itself
  before handing it over.
* The `args` slice is typed `&[&str]`: every argument must already be a `String`. The C# accepts
  any `object`, including numbers and `null`. A `null` argument in the C# renders as the empty
  string; in Rust you pass `""`.
* A placeholder whose index is out of range is left as `{N}` verbatim rather than raising.
* All `RwLock` acquisitions use `.unwrap_or_else(|error| error.into_inner())`, so a poisoned lock
  (from a panicking listener) does not cascade into a panic on every subsequent log call.
* The broadcast channel is lossy by design: a receiver that lags behind gets
  `RecvError::Lagged` and misses messages. `BROADCAST_CAPACITY = 1024` is the ring size.

#### `crates/steamkit/src/util/debug_network_listener.rs` — 159 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/DebugNetworkListener.cs`
**Purpose:** Dumps every packet that goes over the wire to and from the CM server into a directory
of raw `.bin` files, so that a session can be replayed and analysed offline. It is a debugging aid,
not a business-logic component.

**How it works:** The trait `IDebugNetworkListener` has the same two methods as the C#:
`on_incoming_network_message(&self, msg_type: EMsg, data: &[u8])` and
`on_outgoing_network_message(&self, msg_type: EMsg, data: &[u8])`. The implementation
`NetHookNetworkListener` holds a `Arc<dyn ILogContext>`, the target `PathBuf` and an
`AtomicU64` counter.

* `NetHookNetworkListener::new(log)` creates `<exe dir>/nethook/<unix seconds>/` with
  `std::fs::create_dir_all`, and logs `"Created nethook directory: {path}"` under the category
  `"NetHook"`. This is the C# `AppContext.BaseDirectory` + `"nethook"` + `DateTimeToUnixTime
  (DateTime.Now)` layout.
* `NetHookNetworkListener::with_path(path, log)` uses an existing directory and returns
  `SteamError::NotFound("{path} does not exist.")` when it is not a directory — the C# throws
  `DirectoryNotFoundException` with the same message.
* `log_net_message(direction, msg_type, data)` writes the raw bytes with `std::fs::write`; a
  write failure is logged, not propagated (the C# `File.WriteAllBytes` would throw).
* `file_name(direction, msg_type)` produces the on-disk name:
  `"{number:03}_{direction}_{emsg_number}_k_EMsg{EMsgName}.bin"`, e.g.
  `001_in_2_k_EMsgClientLogon.bin`. The number is `message_number.fetch_add(1) + 1`, so it is
  1-based and zero-padded to three digits; `direction` is `"in"` or `"out"`; the EMsg number is
  `msg_type.value()`; the name part is the Rust `Debug` rendering of the `EMsg` variant.

**Notes and gotchas:**
* **This is the on-disk contract with the `steamkit-nethook-analyzer` tool.** The analyzer
  (`crates/steamkit-tools/src/nethook/item.rs`, a port of `NetHookAnalyzer2/NetHookItem.cs`)
  locates the parts of the name with the equivalent of the C# regular expression
  `(?<num>\d+)_(?<direction>in|out)_(?<emsg>\d+)` (case-insensitive) and **ignores everything
  after the EMsg number** — the message name and the `.bin` extension are decoration. The
  analyzer takes the *leftmost* match anywhere in the file name, so the `{number:03}` prefix is
  what pins the sequence number. Because the Rust listener writes the number first and the
  direction second, the two tools agree on the file layout without any further coordination.
  The analyzer's own test cases use the shorter `001_in_2_ProtoBuf.bin` shape, which shows that
  the trailing `_k_EMsg…` part is genuinely optional.
* **The EMsg number in the file name is the *unflagged* number.** The C# writes
  `(int)msgType` from the `EMsg` enum, which never carries the protobuf bit; the Rust writes
  `msg_type.value()`, likewise unflagged. The protobuf flag `0x8000_0000` is therefore *not*
  visible in the file name, even for protobuf messages. It is still visible in the payload, whose
  protobuf header carries the flagged number.
* **The C# `EMsg` is an `int`-backed enum that can hold any value; the Rust `EMsg` is a closed
  enum.** `EMsg::from_value` returns `Option<EMsg>`, so an unknown message number has to be
  resolved before it can be passed to `on_incoming_network_message`. The analyzer handles the
  same problem by mapping an unknown number to `EMsg::Invalid` and keeping the raw number in
  `raw_emsg`.
* The trait method takes `&[u8]` rather than `&[u8; N]`, and the listener writes the bytes
  verbatim — including any framing. The C# is the same.
* `base_directory()` uses `std::env::current_exe()` and takes its parent, falling back to `"."`
  when the executable path cannot be determined. `AppContext.BaseDirectory` in .NET is the
  directory of the *hosting* assembly, which for a normal application is the same thing but
  differs for a library loaded into another process.
* The listener is *not* called for `EMsg::Multi`; the CM client unwraps the multi-packet
  container first and logs the inner messages instead (see `steam/cm_client.rs`).
* The directory name is the Unix timestamp of the moment the listener was created, so two
  listeners created in the same second share a directory. The C# has the same property.

#### `crates/steamkit/src/util/hardware_utils.rs` — 618 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/HardwareUtils.cs`
**Purpose:** Collects the machine identity that Steam uses to recognise a machine, and builds the
"well formed" machine `MessageObject` that the logon message carries.

**How it works:** The file contains the `MachineInfoProvider` factory, four providers
(`Default`, `Windows`, `Linux`, `MacOS`), and the `HardwareUtils` entry points.

`MachineInfoProvider::get_default_provider()` returns the provider for the current platform:
`WindowsMachineInfoProvider` on Windows, `MacOSMachineInfoProvider` on macOS,
`LinuxMachineInfoProvider` on Linux, `DefaultMachineInfoProvider` everywhere else. Each provider
implements the `IMachineInfoProvider` trait (which lives outside `util/`, in
`steam/steam_client/configuration/i_machine_info_provider.rs`) with three methods:
`machine_guid()`, `mac_address()` and `disk_id()`, each returning `Option<Vec<u8>>`.

* **`DefaultMachineInfoProvider`** — `machine_guid()` = UTF-8 of `"{machine_name}-SteamKit"`
  (`machine_name` is `COMPUTERNAME` on Windows, `HOSTNAME` elsewhere); `mac_address()` = the
  first address reported by the platform, or the literal bytes `SteamKit-MacAddress`;
  `disk_id()` = the literal bytes `SteamKit-DiskId`.
  `DefaultMachineInfoProvider::get_combined_mac_address(addresses)` pads every address with two
  zero bytes (so a 6-byte hardware address occupies 8), appends two 8-byte zero blocks as
  fallbacks, takes the first two blocks and concatenates them — always 16 bytes.
* **`WindowsMachineInfoProvider`** — `machine_guid()` reads
  `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` by shelling out to
  `reg query … /v MachineGuid` and taking the last whitespace-separated token of the matching
  line; `mac_address()` shells out to PowerShell with
  `Get-CimInstance Win32_NetworkAdapter | Where-Object { $_.PNPDeviceID -like 'PCI\*' -and $_.MACAddress } | Select-Object -First 2 -ExpandProperty MACAddress`
  and feeds the result through `get_combined_mac_address`; `disk_id()` delegates to
  `Win32Helpers::get_boot_disk_serial_number()`.
* **`LinuxMachineInfoProvider`** — `machine_guid()` reads the first of
  `/var/lib/dbus/machine-id`, `/etc/machine-id`, `/etc/hostname`; `mac_address()` reads
  `/sys/class/net/*/address`, keeping only interfaces that have a `device` sub-directory (i.e.
  physical adapters) and falling back to *all* interfaces if that yields nothing;
  `disk_id()` parses `/proc/cmdline` for `root=UUID=` and then `root=PARTUUID=`, and otherwise
  takes the oldest entry of `/dev/disk/by-uuid` (ordered by mtime).
* **`MacOSMachineInfoProvider`** — `machine_guid()` is the macOS platform serial number
  (`mac_helpers::platform_serial_number()`), falling back to the default provider;
  `mac_address()` returns `None` (the C# returns `null` too); `disk_id()` is the media UUID of
  the boot volume (`mac_helpers::media_uuid("/")`), falling back to the default provider.

`generate_machine_id(provider)` builds the `MessageObject`: it creates the three children
`BB3`, `FF2` and `3B3` as *empty* `KeyValue`s, then sets each child's value to the **lower-case
hex SHA-1** of the corresponding raw bytes (`BB3` ← machine GUID, `FF2` ← MAC address,
`3B3` ← disk id). Each field falls back to the default provider when the custom provider returns
`None`. The `333` child is never set, matching the C# ("333 is some sort of user supplied data
and is currently unused"). The message object is then serialised with
`MessageObject::write_to_stream`.

`HardwareUtils::init(provider)` registers the provider in a generation table and spawns a
tokio `spawn_blocking` task that computes the machine id in the background, so it is ready by
the time the client logs on. `HardwareUtils::get_machine_id(provider)` returns the id, or `None`
when `init` was not called for that provider.

**Notes and gotchas:**
* **The generation table is the port of the C# `ConditionalWeakTable<IMachineInfoProvider,
  Task<MachineID>>`.** The Rust table is a `Mutex<Vec<GenerationEntry>>`, where each entry holds a
  `Weak<dyn IMachineInfoProvider>` and an `Arc<MachineIdSlot>`; the slot holds a
  `OnceLock<Option<Vec<u8>>>`. The weak reference means a provider that is dropped is evicted
  from the table (`table.retain(|entry| entry.provider.strong_count() > 0)`), exactly like the
  C#'s weak-keyed table. The `OnceLock` provides the "compute once" semantics of the C#'s
  `Task`.
* **`get_machine_id` blocks.** `slot.value.get_or_init(...)` runs the generation on the calling
  thread if it has not started yet, whereas the C# waits on the background `Task` for up to
  **30 seconds** and then returns `null` with the message "Unable to generate machine_id in a
  timely fashion, logons may fail". The Rust version has no timeout: it waits as long as the
  generation takes.
* **`init` needs a tokio runtime.** Without one (`tokio::runtime::Handle::try_current()` fails)
  `init` returns silently and the generation is deferred to the first `get_machine_id` call.
  The C# has no such requirement — `Task.Factory.StartNew` uses the thread pool.
* **The three machine-id fields are hashed, not sent raw.** `get_hex_string` computes
  `Sha1::digest(data)` and then `Utils::encode_hex_string`, i.e. lower-case hex of the SHA-1
  digest. The C# does `SHA1.HashData(data)` then `Utils.EncodeHexString(data)`. Only the
  *digest* is sent; the raw machine GUID never leaves the process.
* **The Windows MAC filter is an approximation.** The C# opens
  `SYSTEM\CurrentControlSet\Control\Network\{4D36E972-E325-11CE-BFC1-08002BE10318}\<adapter
  id>\Connection` for every adapter and keeps those whose `PnpInstanceID` starts with `"PCI"`.
  The Rust version asks WMI for `Win32_NetworkAdapter` rows whose `PNPDeviceID` matches
  `PCI\*`. Same intent, different source; the *order* of the two adapters is not guaranteed to
  match the C#'s, and the order matters because the two 8-byte blocks are concatenated in order.
* The Windows `MachineGuid` is read from the 64-bit registry view in the C#
  (`RegistryView.Registry64`); `reg query` on 64-bit Windows opens `HKLM\SOFTWARE` in the 64-bit
  view by default, so this matches. A 32-bit process would see a different value.
* `set_child_value` finds the child with `eq_ignore_ascii_case`, whereas the C#'s
  `this.KeyValues["BB3"]` is an ordinal dictionary lookup. Since the three names are literals
  this makes no difference today, but a case-insensitive match would silently rewrite a
  differently-cased child if the set ever changed.
* `DefaultMachineInfoProvider::get_combined_mac_address` **always** returns 16 bytes even when
  zero or one adapter was found, because of the two 8-byte zero fallbacks. The C# asserts
  `result.Length == 16` with `Debug.Assert`; the Rust port has no equivalent assert.
* The macOS provider deliberately returns `None` for the MAC address, so the fallback provider's
  `SteamKit-MacAddress` constant is used for `FF2` — the same as the C#.

#### `crates/steamkit/src/util/i_log_context.rs` — 38 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/ILogContext.cs`
**Purpose:** The indirection that lets a component write to the debug log *in the context of a
particular client*, instead of always writing to the global log.

**How it works:** The trait `ILogContext` has two methods: `log_debug(&self, category, message)`
and `log_debug_args(&self, category, message, args: &[&str])`, the latter with a default
implementation that calls `format_with_args` (from `util/debug_log.rs`) and forwards the result
to `log_debug`. `DebugLogContext` is the default implementation; it writes to the global
`DebugLog::write_line`. `DebugLogContext::instance()` returns a process-wide `Arc<dyn
ILogContext>` from a `OnceLock`.

**Notes and gotchas:**
* The C# interface is `ILogContext.LogDebug(string category, string message, params object?[]?
  args)` — a single method with an optional argument array. The Rust splits it into two methods,
  because Rust has no optional parameters. **Call sites that pass arguments must use
  `log_debug_args`**; `log_debug` does not format anything.
* As with `DebugLog::write_line_args`, the Rust `args` are `&[&str]`, so .NET format specifiers
  (`{0:X}`) are not supported and are left verbatim in the output.
* `DebugLogContext::instance()` is `pub(crate)`, so the default log context is not part of the
  crate's public API; `NetHookNetworkListener::new` uses it when no context is supplied.

#### `crates/steamkit/src/util/key_dictionary.rs` — 84 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/KeyDictionary.cs`
**Purpose:** Holds the RSA public keys that Steam uses for each `EUniverse`, and hands them out
to the login handshake.

**How it works:** Four `static` byte arrays — `PUBLIC_KEY`, `BETA_KEY`, `INTERNAL_KEY`,
`DEV_KEY` — each 159 bytes long and copied **verbatim** from the C# `KeyDictionary`. They are
DER-encoded `SubjectPublicKeyInfo` structures (a 1024-bit RSA modulus with the
`1.2.840.113549.1.1.1` OID). `KeyDictionary::get_public_key(e_universe: EUniverse) ->
Option<&'static [u8]>` matches on the universe: `Invalid => None`, and the four real universes
return their key. The free function `get_public_key` forwards to it.

**Notes and gotchas:**
* The C# stores the same four keys in a `Dictionary<EUniverse, byte[]?>`, keyed by `EUniverse`;
  the Rust version uses a `match` on the generated `EUniverse` enum, which the compiler checks
  exhaustively. A new universe added to the protobuf definitions therefore produces a
  *compile* error here rather than a `KeyNotFoundException` at runtime.
* The C# dictionary contains only `EUniverse.Public` and `EUniverse.Internal` (the two that the
  Steam language defines as key-bearing); the Rust port also carries `Beta` and `Dev` keys. The
  byte material is the same as the C# — a single wrong byte breaks the login handshake, so do not
  "reformat" these arrays.
* The key is a **DER `SubjectPublicKeyInfo`**, not a bare PKCS#1 `RSAPublicKey` and not a
  modulus-and-exponent pair. `get_public_key` therefore returns it verbatim, exactly like the
  C#; the parsing happens in the networking layer
  (`networking/steam3/envelope_encrypted_connection.rs`).
* The keys are returned as `&'static [u8]`, so there is no allocation and no way for a caller to
  mutate them.

#### `crates/steamkit/src/util/lzma_decoder.rs` — 493 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LzmaDecoder.cs`
**Purpose:** Decodes a raw LZMA stream whose uncompressed size is known up front. This is the
**only** LZMA decoder in the port, and it is SteamKit's own hand-written implementation — it is
not part of the vendored SDK (see the `util/lzma/` intro above).

**How it works:** `LzmaDecoder::decode(properties: &[u8], input: &[u8], output: &mut [u8],
preset_dictionary_length: usize) -> Result<(), SteamError>` is the entry point.

*Validation.* `preset_dictionary_length` must not exceed `output.len()`; `properties` must be at
least `PROPERTIES_SIZE = 5` bytes; and `properties[0]` must be `< 9 * 5 * 5`. The property byte
is unpacked as `lc = byte % 9`, `lp = (byte / 9) % 5`, `pb = byte / 45`. The four dictionary-size
bytes (`properties[1..5]`) are **ignored**: the output buffer is the dictionary.

*Probability array.* All models live in one `Vec<u16>` of
`LITERAL + (LITERAL_CODER_SIZE << (lc + lp))` entries, every entry initialised to
`INITIAL_PROBABILITY = 1 << 10 = 1024`. The model offsets are computed from the same constants
the C# uses: `IS_MATCH = 0`, `IS_REP = 0 + (12 << 4) = 192`, `IS_REP_G0 = 204`, `IS_REP_G1 = 216`,
`IS_REP_G2 = 228`, `IS_REP0_LONG = 240`, `POS_SLOT = 240 + (12 << 4) = 432`,
`SPEC_POS = 432 + (4 << 6) = 688`, `ALIGN = 688 + 128 - 14 = 802`,
`MATCH_LENGTH = 802 + 16 = 818`, `REP_LENGTH = 818 + 514 = 1332`, `LITERAL = 1332 + 514 = 1846`.
The length coder is split into `LENGTH_CHOICE = 0`, `LENGTH_CHOICE2 = 1`,
`LENGTH_LOW = 2`, `LENGTH_MID = 130`, `LENGTH_HIGH = 258`, `LENGTH_CODER_SIZE = 514`.

*The range decoder* (`struct RangeDecoder`) holds `range: u32` (initially `u32::MAX`) and
`code: u32` (initially the big-endian `u32` from `input[1..5]`; the first byte is a discarded
marker). `decode_bit` computes `bound = (range >> 11) * prob` and either takes the lower
interval (`code < bound`: `range = bound`, `prob += (2048 - prob) >> 5`, bit 0) or the upper one
(`range -= bound`, `code -= bound`, `prob -= prob >> 5`, bit 1), renormalising with
`normalize()` (shift `range` left by 8, pull one byte into `code`) whenever
`range < TOP_VALUE = 1 << 24`.

`decode_tree_bit` is the **branchless** variant of the same operation: it derives a
`zero_mask` from the sign bit of `(code as u64).wrapping_sub(bound as u64) >> 63`, uses it to
select the interval without a branch, and returns `one_mask & 1`.

*The main loop* (`decode_stream`) decodes until `position == output.len()`:
`pos_state = position & pos_state_mask`; `state_index = (state << 4) + pos_state`.
`IS_MATCH[state_index] == 0` means a literal. The literal context is
`((position & literal_pos_mask) << lc) + (previous_byte >> (8 - lc))`, and the coder for it
starts at `LITERAL + 0x300 * literal_state`. When `state < 7` the literal is coded plainly
(`decode_literal`); otherwise it is a **matched literal** (`decode_matched_literal`) and needs
the byte at `position - rep0 - 1`.

Otherwise it is a match. `IS_REP[state] == 0` means a *new* match: the rep distances are shifted
(`rep3 = rep2; rep2 = rep1; rep1 = rep0`), the length is decoded from the `MATCH_LENGTH` model
block, the state becomes 7 or 10, and the distance is decoded by `decode_distance`. A distance
of `u32::MAX` is the end marker and is an error here (the size is known up front, so the stream
must fill the buffer exactly). `IS_REP[state] == 1` means a *repeated* match: `IS_REP_G0`
selects between `rep0` and the older distances, and `IS_REP0_LONG[state_index] == 0` with
`IS_REP_G0 == 0` means a one-byte "short rep".

*Length decoding* (`decode_length`) returns the length **minus 2** (the minimum match length):
`LENGTH_CHOICE == 0` selects a 3-bit low tree, `LENGTH_CHOICE2 == 0` selects an 8+3-bit mid
tree, otherwise a 16+8-bit high tree. The caller adds the 2 back.

*Distance decoding* (`decode_distance`) is the pos-slot scheme. `length_state = min(length, 3)`
selects one of four 6-bit pos-slot trees at `POS_SLOT + (length_state << 6)`. A `pos_slot < 4`
means the distance is `pos_slot + 1` outright. Otherwise
`direct_bits = (pos_slot >> 1) - 1` and `distance = (2 | (pos_slot & 1)) << direct_bits` is the
base of the slot's range. For `pos_slot < END_POS_MODEL_INDEX = 14` the remainder is coded by a
reverse bit tree whose base is `SPEC_POS + distance - pos_slot - 1`; for `pos_slot >= 14` the
high bits are coded raw (`decode_direct_bits`) and the low 4 bits by the shared alignment tree at
`ALIGN`.

*The match copy* distinguishes three cases: a non-overlapping copy
(`output.copy_within(source..source + len, position)`), a run of one repeated byte
(`match_distance == 1`), and an overlapping copy done byte-by-byte from the front.

**Notes and gotchas:**
* **Port bug caught by the test suite (bug #3 in §12 of this guide).** In
  `decode_matched_literal` the C# computes `offset ^= previousOffset & (bit - 1);` where `bit`
  is a **`uint`**. When the decoded bit is `0`, `bit - 1` underflows to `0xFFFFFFFF`, so the
  mask keeps all of `previousOffset` and the offset is XORed with it; when the bit is `1`,
  `bit - 1` is `0` and the offset is left alone. The Rust port originally wrote `bit - 1` on a
  `u32`, which **panics in debug builds** on the `0 - 1` underflow, breaking every LZMA and VZip
  decode. The fix is `bit.wrapping_sub(1)`. This is the canonical example of "the C# computes on
  an unsigned type, so it wraps; Rust panics".
* **`decode_tree_bit`'s branchless probability update only works because the probability is a
  `u16`.** `PROBABILITY_ZERO_OFFSET = (1 << MOVE_BITS) - 1 - (1 << BIT_MODEL_TOTAL_BITS) = 31 -
  2048 = -2017` is added to the probability when the bit is zero, and the result is shifted right
  by 5 *logically* (not arithmetically) and subtracted. The intermediate `u32` is huge and
  meaningless; only its **low 16 bits** are correct, and the cast `as u16` is what makes the
  branchless form equal to the C#'s `prob += (2048 - prob) >> 5`. Do not "widen" this to `u32`
  without re-deriving it.
* The C# decoder has a `stackalloc` fast path when `lc + lp <= 4` and only rents from the
  `ArrayPool` otherwise. The Rust port **always** heap-allocates the probability array, which is
  `1846 + 0x300 * 2^(lc+lp)` `u16`s — about 3.7 KB for the default `lc=3, lp=0`.
* The decoder **never checks that the whole input was consumed**, and it errors if the end
  marker arrives before the output buffer is full ("LZMA stream ended before the expected
  uncompressed size"). The C# behaves the same way.
* The C# `Decode` has an optional fourth parameter `presetDictionaryLength = 0`; the Rust makes
  it mandatory. The VZip delta path is the only caller that uses it.
* The C# `DecodeDistance` computes the special-position base as
  `SpecPos + (int)distance - (int)posSlot - 1` — **signed** `int` arithmetic. The Rust writes
  `SPEC_POS + distance as usize - pos_slot as usize - 1`, which is `usize` arithmetic but
  evaluates left-to-right and cannot underflow, because `SPEC_POS` is large and `distance >=
  pos_slot` always holds for `pos_slot >= 4`.
* Match distances are validated (`rep0 as usize >= position` is an error) because the output
  buffer is the dictionary; a malicious stream cannot read outside it.
* The C# `LzmaDecoder` is `internal static`; the Rust `LzmaDecoder` is `pub` because
  `util::lzma_util` and `util::v_zip_util` are `pub`.

#### `crates/steamkit/src/util/lzma_util.rs` — 119 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LzmaUtil.cs`
**Purpose:** Decodes Steam's "LZMA"-magic-prefixed blobs — the framing that wraps an LZMA stream
with its uncompressed and compressed lengths.

**How it works:** `LZMA_MAGIC = 0x4C5A4D41` is the ASCII string `"LZMA"`. The blob layout is:

```
offset  size  meaning
0       4     magic "LZMA", big endian u32
4       4     uncompressed length, little endian i32
8       4     compressed length, little endian i32
12      5     LZMA properties (lc/lp/pb byte + 4-byte dictionary size)
17      n     LZMA stream
```

`LzmaUtil::has_lzma_header(data)` reads the first four bytes as a **big endian** `u32` and
compares them with the magic; it returns `false` for inputs shorter than four bytes.

`LzmaUtil::decompress(input, output, compressed_length, uncompressed_length)` reads the five
property bytes, reads `compressed_length` bytes of compressed data, allocates a
`uncompressed_length` byte buffer, calls `LzmaDecoder::decode(&properties, &compressed, &mut
decompressed, 0)` and finally `output.write_all(&decompressed)`.

`LzmaUtil::try_decompress(input, output_factory)` reads the magic, the uncompressed length and
the compressed length; if the magic does not match it returns `Ok(None)`. Negative lengths are an
error. It then asks `output_factory` for a buffer of the right size, **empties it** (`output
.clear()`), runs `decompress`, and returns `Ok(Some(output))`.

**Notes and gotchas:**
* **Port bug caught by the test suite (bug #9 in §12 of this guide).** The
  C# `TryDecompress` hands the output to a factory that returns a `MemoryStream` with the
  **capacity but no content**, and `MemoryStream.Write` writes at position 0; the C# then seeks
  back to 0 so the stream holds exactly the decompressed bytes. In Rust the factory returns a
  `Vec<u8>`, and `write_all` on a `Vec` **appends at the end**. When the factory pre-filled the
  vector (the natural `vec![0u8; len]`), the decoded data landed *after* the pre-fill: the result
  was exactly twice the expected length, with the content at the end. The fix is the explicit
  `output.clear()` before decompressing — the Rust equivalent of "the stream has capacity but no
  content". **This is the single most important thing to know about this module**: any new
  `output_factory` must either return an empty `Vec` or be paired with the `clear()`.
* `try_decompress` **consumes 4 bytes of the input even when it returns `Ok(None)`** (it has to
  read the magic to know). The C# has the same behaviour. `has_lzma_header` does *not* consume
  anything.
* The magic is read **big endian** but the two lengths are read **little endian**. That mix is
  the C#'s (`BinaryPrimitives.ReadInt32BigEndian` for the magic,
  `ReadInt32LittleEndian` for the lengths) and is a real trap: it is the only big-endian field
  in the file.
* The lengths are signed `i32` in the C# and are rejected when negative; the Rust reads them as
  `i32` and does the same, then casts to `usize`.
* `decompress` writes exactly `uncompressed_length` bytes to `output`, regardless of what the
  LZMA stream actually produced. `LzmaDecoder::decode` fills the buffer and errors if the stream
  ends early, so the sizes agree.
* `LzmaUtil` has no caller inside the crate at the moment — the depot-chunk path uses the VZip
  and VZstd containers directly, which carry their own framing.

#### `crates/steamkit/src/util/mac_helpers.rs` — 140 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/MacHelpers.cs`
**Purpose:** Reads the macOS platform serial number and the boot volume's media UUID, which the
machine-id provider needs.

**How it works:** The C# file is a set of P/Invoke bindings: `LibC.statfs64`, the CoreFoundation
`CFStringCreateWithCString` / `CFStringGetCString` pair, the DiskArbitration
`DADiskCreateFromBSDName` / `DADiskCopyDescription` pair, and IOKit's
`IOServiceGetMatchingService` / `IORegistryEntryCreateCFProperty`. Because the crate forbids
`unsafe`, none of that can be reproduced.

The Rust port reaches the same values through the command line tools macOS ships:

* `platform_serial_number()` runs `ioreg -rd1 -c IOPlatformExpertDevice` and parses the line
  `"IOPlatformSerialNumber" = "C02XXXX"` with `parse_quoted_property`.
* `media_uuid(mount_point)` runs `diskutil info <mount_point>` and parses the `Volume UUID:`
  line with `parse_property_line`. A value of `(null)` or an empty value counts as "not found".
* `run_command` returns `None` when the process cannot be started or exits non-zero.

The constants `K_IO_PLATFORM_SERIAL_NUMBER_KEY = "IOPlatformSerialNumber"` and
`K_DA_DISK_DESCRIPTION_MEDIA_UUID_KEY = "DAMediaUUID"` are kept as named constants so the
property names are greppable. The `StatFS` struct is kept verbatim from the C# — it is the
`statfs` layout the C# hands to `statfs64` — but is marked `#[allow(dead_code)]` because the
Rust port never calls into libc.

**Notes and gotchas:**
* **This is a deliberate, documented deviation, not a port of the P/Invoke code.** The C# reads
  the values through the frameworks; the Rust reads the same properties through `ioreg` and
  `diskutil`. The module header says so explicitly.
* **The disk UUID is not the same value as the C#'s.** The C# reads `DAMediaUUID` — the
  **whole-disk** media UUID — from the disk that carries the mount point
  (`DADiskCreateFromBSDName(…, stat.f_mntfromname)`). The Rust reads the `Volume UUID:` line of
  `diskutil info`, which on modern macOS is the **APFS volume** UUID (`diskutil` also prints a
  separate `Disk UUID:` line). The `3B3` field of the machine id therefore does **not** match
  the C# byte-for-byte on macOS. If byte-exact parity with the C# matters, the `Disk UUID:` line
  is the closer match.
* The C# encodes the macOS values with `Encoding.ASCII.GetBytes`; the Rust uses
  `String::into_bytes`, which is UTF-8. Identical for the ASCII values that macOS reports.
* The commands are spawned unconditionally — there is no `#[cfg(target_os = "macos")]` gate, so
  the module compiles everywhere and simply returns `None` on other platforms (where `ioreg`
  and `diskutil` do not exist).
* `ioreg` output is locale independent for the property names and the quoting, which is why the
  parser looks for a leading `"` rather than for a translated label.
* The fallback in `hardware_utils.rs` keeps the machine id **stable** across runs of the same
  machine: when `ioreg`/`diskutil` fail, the default provider's constant is used, so the id does
  not change from one run to the next.

#### `crates/steamkit/src/util/msg_util.rs` — 93 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/MsgUtil.cs`
**Purpose:** Manipulates Steam's message numbers — adding, stripping and testing the protobuf
flag, and converting between the numeric wire value and the `EMsg` enum.

**How it works:** Two constants: `PROTO_MASK = 0x8000_0000` (the protobuf flag) and
`EMSG_MASK = !PROTO_MASK = 0x7FFF_FFFF`.

* `MsgUtil::make_msg(msg: EMsg, protobuf: bool) -> u32` returns `msg.value() as u32`, OR-ed with
  `PROTO_MASK` when `protobuf` is true.
* `MsgUtil::make_gc_msg(msg: u32, protobuf: bool) -> u32` does the same for the game
  coordinator message numbers, which are not part of the `EMsg` enum.
* `MsgUtil::get_msg(msg: u32) -> Option<EMsg>` masks off the flag and maps the number back with
  `EMsg::from_value`.
* `MsgUtil::get_gc_msg(msg: u32) -> u32` returns `msg & EMSG_MASK`.
* `MsgUtil::is_proto_buf(msg: u32) -> bool` is `(msg & PROTO_MASK) > 0`.
* The `EMsg`-typed overloads `get_msg_from`, `is_proto_buf_from` mirror the C#
  `GetMsg(EMsg)` and `IsProtoBuf(EMsg)`.

**Notes and gotchas:**
* **The `0x8000_0000` bit is Steam's "this message carries a protobuf body" flag.** It is set on
  the wire message number of every protobuf-backed message and cleared on the old
  `KeyValue`-serialised ones. `make_msg`/`make_gc_msg` set it, `get_msg`/`get_gc_msg` strip it.
* **The C# `MakeMsg` returns an `EMsg`, the Rust returns a `u32`.** The C# `EMsg` is an
  `int`-backed enum, so `(EMsg)(msg | ProtoMask)` happily stores a value that is not a declared
  member. A Rust `EMsg` is a **closed** enum and cannot carry the flag at all, so the port
  returns the raw `u32`. The same applies to `GetMsg(uint)`: the C# returns
  `(EMsg)(msg & EMsgMask)`, which is an *undeclared* enum value when the number is unknown; the
  Rust returns `None`.
* **`is_proto_buf_from(msg: EMsg)` is always `false`.** A Rust `EMsg` can never carry the
  protobuf bit, so the method is only useful for symmetry with the C# API. To test the flag you
  must use the raw `u32` overload.
* The C# also has `GetMsg(EMsg)`; the Rust names it `get_msg_from` to avoid an overload that
  Rust cannot express (two `get_msg` functions with different argument types cannot coexist in
  the same module without renaming).
* `EMSG_MASK` is `!PROTO_MASK` rather than `0x7FFF_FFFF`, matching the C#'s `~ProtoMask`
  exactly.
* The `EMsg` enum lives in `base/generated/steam_language.rs`, which is generated from the
  `steam_language` protobuf definitions.

#### `crates/steamkit/src/util/net_helpers.rs` — 106 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/NetHelpers.cs`
**Purpose:** Helpers for converting between IP addresses, `SocketAddr`s and Steam's
`CMsgIpAddress` protobuf message, including the obfuscation Steam applies to the client's own
private address in the logon request.

**How it works:**

* `get_local_ip(local_end_point: Option<SocketAddr>) -> Result<IpAddr, SteamError>` returns the
  local address, and `SteamError::Connection("Socket not connected")` when the endpoint is
  missing or unspecified.
* `get_ip_address(ip_addr: u32) -> IpAddr` and `get_ip_address_as_uint(&IpAddr) -> Result<u32,
  SteamError>` convert between a packed IPv4 number and an `IpAddr`. **The packed number is in
  network (big endian) byte order**: `get_ip_address(0x0102_0304) == 1.2.3.4` and
  `get_ip_address_as_uint(1.2.3.4) == 0x0102_0304`. The IPv6 case is rejected with
  `SteamError::InvalidArgument("GetIPAddressAsUInt only works with IPv4 addresses.")`.
* `get_ip_address_from_msg(&CMsgIpAddress) -> Option<IpAddr>` prefers the IPv6 form when the
  `ip` oneof carries it; an IPv6 blob that is not exactly 16 bytes yields `None`.
* `get_msg_ip_address(&IpAddr) -> CMsgIpAddress` is the inverse.
* `obfuscate_private_ip(&mut CMsgIpAddress)` XORs the IPv6 bytes with the repeating key
  `0D F0 AD BA` and the IPv4 value with `MsgClientLogon::OBFUSCATION_MASK`. The message is
  modified in place, exactly like the C# extension method, which mutates and returns the same
  object.
* `try_parse_ip_end_point(&str) -> Option<SocketAddr>` parses `address:port` and rejects port 0.
* `extract_endpoint_host(&SocketAddr) -> String` and `extract_endpoint_port(&SocketAddr) -> u16`
  split an endpoint.

**Notes and gotchas:**
* **The packed IPv4 number is in network byte order on both sides.** The C#
  `GetIPAddress(uint)` expression looks like a byte swap but is arithmetically the **identity**
  (`(x & 0xFF000000) >> 24 | (x & 0x00FF0000) >> 8 | (x & 0x0000FF00) << 8 | (x & 0x000000FF)
  << 24` re-assembles the original word), and `new IPAddress(long)` reads its argument in network
  order; the two cancel out. The C# `GetIPAddressAsUInt` goes through `TryWriteBytes` plus
  `NetworkToHostOrder`, which also yields network order. The Rust uses `Ipv4Addr::from(u32)` and
  `u32::from_be_bytes`, which are the natural Rust spellings of "network order".
* **The C# takes a `Socket`, the Rust takes the local end point.** `GetLocalIP(Socket)` reads
  `socket.LocalEndPoint`, casts it to `IPEndPoint` and throws `InvalidOperationException("Socket
  not connected")` when the cast fails or the address is `IPAddress.Any`. The Rust takes
  `Option<SocketAddr>` and reproduces the "not connected" case as an unspecified address.
* **The IPv6 obfuscation is the same in both.** The C# writes out sixteen explicit XOR lines
  (`v6[0] ^= 0x0D; … v6[15] ^= 0xBA;`) because the `v6` field is exactly 16 bytes; the Rust
  loops over the bytes with `KEY[index % KEY.len()]`, which produces the identical result.
* The C# `GetIPAddress(CMsgIPAddress)` falls back to `v4` (i.e. `0.0.0.0`) when the `v6` field
  is absent; the Rust returns `None` when the `ip` oneof is unset, because a `CMsgIpAddress`
  with no `ip` is a different thing from an address of `0.0.0.0`.
* `extract_endpoint_port` returns `u16` in Rust and `int` in C#; a port never exceeds 65535, so
  the conversion is lossless.
* The C# `ExtractEndpointHost` handles a `DnsEndPoint` by returning the host name. A Rust
  `SocketAddr` always holds a resolved IP, so the host part is always the numeric address. This
  is a consequence of `SUMMARY.md` §8, which freezes the connection API to `SocketAddr`.

#### `crates/steamkit/src/util/scheduled_function.rs` — 144 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/ScheduledFunction.cs`
**Purpose:** Runs an action on a fixed schedule — immediately, and then every `Delay` — and
stops it again. SteamKit uses this kind of timer for the periodic heartbeat and the
reconnect timer.

**How it works:** The state lives in a `ScheduledFunctionState` shared with the spawned task:
`delay: RwLock<Option<Duration>>` (`None` = fire once), `running: AtomicBool`, and
`cancel: Mutex<Option<CancellationToken>>`. The action itself is an
`Arc<dyn Fn() + Send + Sync>`.

* `ScheduledFunction::new(func)` is the "fire once" constructor; `with_delay(func, Some(d))` is
  the periodic one.
* `start()` is a no-op when the function is already running. It requires a tokio runtime
  (`tokio::runtime::Handle::try_current()`); without one it logs
  `"Unable to start a scheduled function outside of a tokio runtime."` under the category
  `"ScheduledFunction"` and resets the running flag. Otherwise it creates a
  `CancellationToken`, stores it, and spawns a task that loops over
  `tokio::time::interval(period)` with `MissedTickBehavior::Delay`, selecting between
  `token.cancelled()` and `interval.tick()`. `tokio::time::interval` fires its first tick
  immediately, which reproduces the C#'s `timer.Change(TimeSpan.Zero, Delay)`.
* The "fire once" case uses a `biased` select between `token.cancelled()` and
  `std::future::ready(())`, so the action runs exactly once (unless the token was already
  cancelled).
* `stop()` cancels the token. `Drop` calls `stop()`.

**Notes and gotchas:**
* **The C# expresses "fire once" as a delay of `-1` milliseconds** (`Timeout.Infinite`) in the
  `Timer` constructor; the Rust uses `Option<Duration>` because `Duration` cannot be negative.
* **The C# timer runs the callback on the thread pool; the Rust runs it on a tokio worker
  thread.** A blocking callback will therefore block the runtime, not just a pool thread.
* `stop()` does **not** wait for an in-flight tick to finish — it only signals cancellation. The
  C#'s `timer.Change(Timeout.Infinite, Delay)` behaves the same way.
* **`set_delay` while running has no effect until the next `start()`.** The C# has the same
  property: `Delay` is only handed to `timer.Change` in `Start`/`Stop`.
* `MissedTickBehavior::Delay` means a slow callback causes the *next* tick to be delayed rather
  than a burst of catch-up ticks (tokio's default is `Burst`). This is a deliberate difference
  from `System.Threading.Timer`, which also skips missed ticks.
* The Rust `Drop` implementation cancels the token, which stops the spawned task; the C#'s
  finalizer (`~ScheduledFunction`) calls `Stop()`, which is the same idea but runs at an
  unpredictable time.
* `ScheduledFunction` is `pub(crate)` and currently has no consumer in the crate.

#### `crates/steamkit/src/util/simple_console_debug_listener.rs` — 27 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/SimpleConsoleDebugListener.cs`
**Purpose:** A debug listener that prints to stdout — the "just show me the log" listener.

**How it works:** `SimpleConsoleDebugListener` implements `IDebugListener::write_line(&self,
category, msg)` with `println!("[{category}]: {msg}")`, which is the direct translation of the
C# `Console.WriteLine("[{0}]: {1}", category, msg)`. It also offers `new()` and a `Default`
implementation.

**Notes and gotchas:**
* The format is exactly `[category]: message`, with a colon and a space after the closing
  bracket — the same as the C#, so log lines are byte-identical.
* The C# uses `Console.WriteLine` with a composite format; the Rust uses `println!` with inline
  captures. The C#'s `Console.WriteLine` goes to `Console.Out` (which can be redirected); the
  Rust's `println!` always goes to the process's stdout.
* `SimpleConsoleDebugListener` is `pub` and is registered with
  `DebugLog::add_listener(Arc::new(SimpleConsoleDebugListener::new()))`.
* One of the four deliberately ignored tests in the suite lives in this module (the ported C#
  test suite has a helper here that could not be expressed in Rust).

#### `crates/steamkit/src/util/steam_kit_web_request_exception.rs` — 119 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/SteamKitWebRequestException.cs`
**Purpose:** The error type raised when an HTTP request fails, carrying the status code and the
response headers.

**How it works:** `SteamKitWebRequestException` holds a `message: String`, a
`status_code: reqwest::StatusCode`, a `headers: reqwest::header::HeaderMap` and an optional
`source: Option<Box<dyn std::error::Error + Send + Sync>>`. Three constructors:
`new(message, &Response)` copies the status and headers off a live response;
`from_message(message)` and `with_source(message, source)` produce an exception with
`StatusCode::OK` and no headers. `status_code()`, `headers()` and `message()` are the accessors.
`Display` renders `"{message} (HTTP {code})"`, `Debug` renders a `debug_struct` with the same
fields, and `std::error::Error::source` returns the inner error. `From<SteamKitWebRequestException>
for SteamError` maps it onto `SteamError::Other(value.to_string())`.

**Notes and gotchas:**
* **The C# derives from `HttpRequestException`** and therefore *is* an `HttpRequestException`
  for every `catch` block that tests for it. The Rust type is standalone and is **not** a
  `reqwest::Error`, so `catch` clauses that test for `HttpRequestException` have to be
  rewritten to test for `SteamKitWebRequestException` (or for `SteamError`).
* The C# exposes `public new HttpStatusCode StatusCode => base.StatusCode ?? default;` — i.e.
  **0** when the exception was not built from a response. The Rust uses `StatusCode::OK` (200)
  for the message-only constructors. There is no `HttpStatusCode` value with the numeric value
  0 in `reqwest` (`StatusCode` is an enum of valid HTTP codes), so `OK` is the closest
  available stand-in. Do not read the status code of a `from_message` exception as meaningful.
* The C#'s `Headers` property is an `HttpResponseHeaders` and is non-null; for the
  message-only constructors it is the default (empty) instance. The Rust's `HeaderMap::new()`
  matches that.
* `SteamKitWebRequestException` is the C# counterpart of the WebAPI error type
  (`steam/webapi/web_api_request_exception.rs` is the other half of that pair); the two are
  separate in both trees.

#### `crates/steamkit/src/util/stream_helpers.rs` — 222 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/StreamHelpers.cs`
**Purpose:** Extension methods for reading and writing the little-endian primitives and
null-terminated strings that Steam's binary protocols use.

**How it works:** The C# file provides synchronous extension methods on `System.IO.Stream`; the
Rust port provides the same operations as **asynchronous** extension traits over
`tokio::io::AsyncRead` / `AsyncWrite`, implemented with `#[async_trait]` and blanket-implemented
for every `R: AsyncRead + Send + Unpin` (resp. `AsyncWrite`).

* `StreamReadHelpers` provides `read_int16`, `read_uint16`, `read_int32`, `read_int64`,
  `read_uint32`, `read_uint64` and `read_float`. Each one does an `read_exact` into a
  fixed-size array and then `iN::from_le_bytes` / `uN::from_le_bytes` / `f32::from_le_bytes`.
  **Every multi-byte number is little endian**, which is what .NET's `BitConverter` does on the
  little-endian platforms Steam runs on.
* `read_null_term_string(encoding)` reads up to the next `0` byte. For UTF-8 it reads one byte
  at a time (the terminator is always a single zero byte); for the single-byte encodings it
  reads a whole character at a time and stops at the end of the stream.
* `read_all(&mut [u8]) -> Result<usize>` fills the buffer as far as the stream allows and
  returns the number of bytes read.
* `StreamWriteHelpers::write_null_term_string(value, encoding)` encodes the string, appends a
  `0` byte and flushes.

`StreamEncoding` is a closed enum with `Utf8`, `Ascii` and `Latin1`. `Ascii` maps code points
above `0x7F` to `?`; `Latin1` maps code points above `0xFF` to `?`; `Utf8` uses
`String::from_utf8_lossy` on the way back.

**Notes and gotchas:**
* **The C# takes a `System.Text.Encoding`, the Rust takes a closed enum.** .NET's
  `Encoding.Unicode` (UTF-16) and `Encoding.BigEndianUnicode` have no counterpart here; UTF-16
  is handled elsewhere, in the KeyValue text reader, which sniffs the byte-order mark.
* `StreamEncoding::character_size()` always returns `1`. Every encoding in the enum except
  UTF-8 is single byte, and UTF-8 is handled by the separate byte-at-a-time path, so the
  indirection is vestigial — but the multi-byte read path still uses it, so it is not simply
  dead code.
* The .NET encoders use the default replacement fallback, which is `?` for `Encoding.ASCII` and
  `Encoding.Latin1`; the Rust matches that exactly. `Encoding.UTF8` uses `U+FFFD`, which
  `String::from_utf8_lossy` also produces.
* `read_exact` returns an error on a short read; the C#'s `ReadExactly` throws
  `EndOfStreamException`. The Rust error is a `std::io::Error` wrapped in `SteamError`.
* `read_all` corresponds to the C# `ReadAll(this Stream, Span<byte>)`, which uses
  `ReadAtLeast(buffer.Length)` — i.e. it reads *at least* the buffer length or throws. The Rust
  version returns the count instead of throwing on a short read, which is the more useful
  behaviour but is not identical.
* The traits are blanket-implemented, so `use crate::util::stream_helpers::StreamReadHelpers;`
  is enough to get every method on any `AsyncRead`.
* `stream_helpers` has no consumer inside the crate at the moment.

#### `crates/steamkit/src/util/task_extensions.rs` — 81 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/TaskExtensions.cs`
**Purpose:** Awaits a task and swallows its cancellation error when the token has been
cancelled, so that a deliberately-cancelled background task does not surface as an error.

**How it works:** `IgnoringCancellation<F>` is a `Future` adapter holding a
`Pin<Box<F>>` and a `CancellationToken`. Its `poll` forwards to the inner future; on
`Poll::Ready(Err(error))` it checks `matches!(error, SteamError::Cancelled) && token.is_cancelled()`
and converts that specific combination into `Poll::Ready(Ok(()))`. Everything else is passed
through unchanged. `ignoring_cancellation(task, token)` is the free-function form, and the
`TaskExtensions` trait offers the same thing as an extension method
(`future.ignoring_cancellation(token)`); the trait is blanket-implemented for every
`F: Future<Output = Result<(), SteamError>> + Sized`.

**Notes and gotchas:**
* The C# is
  `try { await task.ConfigureAwait(false); } catch (OperationCanceledException) when (token.IsCancellationRequested) { }`.
  The Rust matches on `SteamError::Cancelled` rather than on a dedicated exception type, so the
  inner future has to surface cancellation as `SteamError::Cancelled` for the swallow to happen.
  A future that returns a *different* error on cancellation is **not** swallowed.
* **Only the cancellation error is swallowed.** The C# catches `OperationCanceledException`; any
  other exception propagates. The Rust is equivalent.
* `ConfigureAwait(false)` has no Rust analogue — a Rust future is not tied to a
  synchronization context, so this is a non-issue.
* The inner future is **boxed** (`Box::pin`), which costs one allocation per wrapper. The C#
  boxes nothing.
* The trait method takes `self` by value, so it consumes the future; the free function takes it
  by value as well.

#### `crates/steamkit/src/util/utils.rs` — 234 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/Utils.cs`
**Purpose:** Hex encoding/decoding and the detection of the operating system, which Steam uses to
fill the `EOSType` field of the logon request.

**How it works:**

* `Utils::encode_hex_string(&[u8]) -> String` is `hex::encode`, i.e. **lower case** hex — the
  equivalent of .NET's `Convert.ToHexString(input).ToLowerInvariant()`.
* `Utils::decode_hex_string(Option<&str>) -> Result<Option<Vec<u8>>, SteamError>` returns `None`
  for a `None` input (matching the C#'s nullable `byte[]? DecodeHexString(string? hex)`) and an
  error for a string that is not valid hex, where the C# throws a `FormatException`.
* `Utils::get_os_type() -> EOSType` is cached in a `OnceLock`, because the OS cannot change while
  the process runs.

`detect_os_type()` dispatches on `#[cfg(target_os = …)]`:

* **Windows** — `detect_windows_os_type()` runs `cmd /c ver`, extracts the bracketed version
  (`Microsoft Windows [Version 10.0.19045.4046]` — the bracketed part is locale independent) and
  maps it: major 4 → `WinNT`; major 5 → `Win2000` / `WinXP` / `Win2003` by minor; major 6 →
  `WinVista` / `Windows7` / `Windows8` / `Windows81` by minor; major 10 with build `>= 22000` →
  `Win11`, otherwise `Windows10`; anything else → `WinUnknown`. If the version cannot be read,
  it reports `Windows10` rather than an unknown platform.
* **Linux** — `detect_linux_os_type()` reads `/proc/sys/kernel/osrelease` (which is what
  `Environment.OSVersion.Version` reports on Linux) and maps the kernel version to the
  `Linux22` … `Linux7x` values.
* **macOS** — `detect_macos_os_type()` runs `sw_vers -productVersion` and converts the *product*
  version into the *Darwin* version that `Environment.OSVersion` reports: product 10.x → Darwin
  `minor + 4` (10.7 → 11, 10.15 → 19), product ≥ 11 → Darwin `major + 9` (11 → 20, 15 → 24).
  The Darwin version is then mapped to `MacOS107` … `MacOS15`.
* Anything else → `EOSType::Unknown`.

`parse_version(text)` splits a dotted version string into major/minor/build, ignoring a
non-numeric suffix such as the `-generic` of a Linux kernel release.

**Notes and gotchas:**
* **The C# reads the OS version from the runtime; the Rust reads it from the platform's own
  tooling.** `Environment.OSVersion` and `RuntimeInformation.IsOSPlatform` are not available in
  Rust, so the version is obtained from `cmd /c ver`, `/proc/sys/kernel/osrelease` and
  `sw_vers`. The mappings are the C#'s.
* **The C#'s Windows 9x branch is unreachable on modern .NET.** The C# switches on
  `Environment.OSVersion.Platform`: `Win32Windows` maps minor 0/10/90 to `Win95`/`Win98`/`WinME`,
  and `Win32NT` maps major 4 to `WinNT`. Since .NET 5, `Platform` is always `Win32NT`, so the
  9x branch is dead. The Rust always takes the NT branch, which matches the C#'s behaviour on
  every platform that actually exists today.
* **The Rust falls back to `Windows10` when the Windows version cannot be determined.** The C#
  would have known the real version through `Environment.OSVersion`. The fallback is a
  deliberate choice ("report a modern Windows rather than an unknown platform").
* The macOS product-to-Darwin conversion is the interesting part: `sw_vers` reports `15.2`,
  while `Environment.OSVersion` on macOS reports the Darwin kernel version `24.2`. The
  conversion `major + 9` / `minor + 4` reproduces that mapping exactly.
* The `EOSType` variants are the Valve-defined ones; note that the protobuf enum spells some of
  them inconsistently (`MacOS107` … `MacOS1012` versus `Macos1013` … `Macos1015`).
* `Utils` is `pub(crate)`; the free functions `encode_hex_string`, `decode_hex_string` and
  `get_os_type` are the ones used elsewhere in the crate (17 call sites).

<!-- CONTINUE -->

#### `util/` — the decompression chain, the VZip delta, the stopwatch and the Windows helper

Six files in `crates/steamkit/src/util/` cover the machinery that turns the bytes of a depot chunk into the bytes of a file: the decompression chain itself (`v_zstd_util.rs` -> `v_zip_util.rs` -> `zip_util.rs`), the VZip delta decoder that reconstructs a chunk from a previously downloaded one (`v_zip_delta_util.rs`), the monotonic stopwatch used by the job/timeout machinery (`value_stopwatch.rs`), and the Windows-only boot-disk serial helper that feeds the machine fingerprint (`win32_helpers.rs`). All six are ports of files under `csharp-original/SteamKit2/SteamKit2/Util/` — the Rust module tree mirrors the C# namespace tree one-for-one, with the caveat that the C# files are split across two namespaces (`ZipUtil`, `VZipUtil`, `VZstdUtil` and `VZipDeltaUtil` live in `SteamKit2`, while `ValueStopwatch` and `Win32Helpers` live in `SteamKit2.Util`) and the Rust port collapses both into `util` — and the whole chain is exercised end to end by `DepotChunk::process`, which tries zstd, then vzip, then plain zip.

#### `crates/steamkit/src/util/value_stopwatch.rs` — 38 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/ValueStopwatch.cs`
**Purpose:** A minimal monotonic stopwatch that reports how much time has passed since it was started. It exists so that long-running operations can be timed without allocating anything, and it reproduces the C# `ValueStopwatch` exactly, including the rule that an uninitialised stopwatch is an error rather than a zero elapsed time.
**How it works:** `ValueStopwatch` is a `Clone + Copy + Debug` struct with one field, `start_timestamp: Option<Instant>`. `ValueStopwatch::start_new()` sets it to `Some(Instant::now())`. `ValueStopwatch::get_elapsed_time(&self) -> Result<Duration, SteamError>` matches on the option: `Some(start)` yields `Ok(start.elapsed())`, `None` yields `Err(SteamError::InvalidArgument("An uninitialized, or 'default', ValueStopwatch cannot be used to get elapsed time."))` — the C# message verbatim. `impl Default` produces the `None` variant, which is how "not started yet" is represented.
**Notes and gotchas:**
- The C# version is a `readonly struct` that wraps the raw `Stopwatch.GetTimestamp()` counter and converts it to a `TimeSpan` with `s_timestampToTicks = TimeSpan.TicksPerSecond / (double)Stopwatch.Frequency`. The Rust port uses `std::time::Instant` and returns a `Duration`; both are monotonic and high resolution (`Instant` is `QueryPerformanceCounter` on Windows, `CLOCK_MONOTONIC` on Linux/macOS), but the units differ — C# returns 100-ns ticks inside a `TimeSpan`, Rust returns nanoseconds in a `Duration`.
- The C# detects "uninitialised" by testing `_startTimestamp == 0`; the Rust uses `Option`. The C# therefore also treats a stopwatch whose raw timestamp happened to be 0 as uninitialised (in practice unreachable), while the Rust representation has no such edge case.
- The error type differs: the C# throws `InvalidOperationException`, the Rust returns `SteamError::InvalidArgument`. The Rust error path is what makes the type fallible rather than panicking.
- The struct is `pub(crate)`, and in this port it is currently unused: `crates/steamkit/src/types/job_id.rs` keeps its own `job_start: Instant` fields instead, whereas the C# `SteamKit2/Types/JobID.cs` holds a `ValueStopwatch jobStart` in `AsyncJob`, starts it with `ValueStopwatch.StartNew()` in the constructor and reads `jobStart.GetElapsedTime() >= Timeout` in `IsTimedout` (default timeout 10 s). The port therefore has an unused-but-faithful helper rather than a used one.
- `Instant` is opaque, so unlike the C# `long` the Rust stopwatch cannot be serialised or converted to an absolute timestamp; it is still `Copy`, `Send` and `Sync` because `Instant` is.

#### `crates/steamkit/src/util/v_zip_delta_util.rs` — 46 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/VZipDeltaUtil.cs`
**Purpose:** Decodes a "VZip delta" — a VZip stream that was compressed against a chunk the client already has, so that the server only has to ship the difference. The delta is not a separate container format: it is the ordinary VZip frame layout with the version byte `'d'` instead of `'a'`, whose LZMA stream was encoded with the previously downloaded chunk as its preset dictionary.
**How it works:** `VZipDeltaUtil::decompress(buffer: &[u8], source_chunk_data: &[u8]) -> Result<Vec<u8>, SteamError>` does four things:
1. `VZipUtil::read_frame(buffer, VERSION)` with `VERSION = b'd'` parses the header and footer and returns a `Frame` holding the 5 LZMA property bytes, the compressed stream, the footer's `output_crc` and the footer's `decompressed_size`.
2. `total_length = source_chunk_data.len() + frame.decompressed_size as usize`; a `Vec<u8>` of that size is allocated and the source chunk is copied into its front. That buffer is the LZMA window.
3. `LzmaDecoder::decode(frame.properties, frame.compressed, &mut window, source_chunk_data.len())` decodes in place starting at offset `source_chunk_data.len()`, with the preset-dictionary length set to the same value so that matches may reach back into the source chunk.
4. The result is `&window[source_chunk_data.len()..]`; `crc32fast::hash(out_data)` is compared against `frame.output_crc` and a mismatch is reported as `SteamError::Decompression("CRC does not match decompressed data. VZip data may be corrupted.")`.
**Notes and gotchas:**
- There is no separate delta instruction set. The "instructions" are plain LZMA literals and matches; the source/target split is expressed purely through the LZMA window. The encoder was given the source chunk as its trained dictionary (`Encoder::set_train_size(source.len())`), so the matches in the stream reference bytes at negative distances from the start of the target.
- Positions continue *after* the dictionary: the decoder starts at `position = source_chunk_data.len()`, so `pos_state`, `literal_pos` and the previous-byte context are all computed relative to the tail of the source chunk. This mirrors the C# fix in commit `b6582b0a` ("Continue LZMA positions after the trained dictionary in delta chunks"), which changed `VZipDeltaUtil.cs` and the 7-Zip SDK decoder under `SteamKit2/Util/LZMA/Compress/LZMA/`. The Rust decoder that ships with this port is `crates/steamkit/src/util/lzma_decoder.rs` (the port of `SteamKit2/Util/LzmaDecoder.cs`), and its `decode` takes the same `preset_dictionary_length` argument.
- The CRC in the VZip *header* is also the CRC of the target chunk (the C# comment says "The CRC in the header is also the CRC of the chunk"), but this port only checks the *footer* CRC against the decoded output. The header's four ambiguous bytes are deliberately not interpreted by `read_frame` (they are a timestamp for client VZips and a CRC for depot chunks).
- The CRC check is unconditional, as it is in the C#. That is deliberate: decompressing against the wrong source chunk must fail loudly, and `crates/steamkit-tests/tests/lzma.rs::v_zip_delta_throws_with_wrong_source_chunk` asserts that a one-byte-corrupted source yields a `SteamError::Decompression`.
- The C# rents the window from `ArrayPool<byte>.Shared` and returns it in a `finally`; the Rust port allocates a fresh `Vec` and copies the source into it, which costs one extra allocation and one copy of the whole source chunk per call.
- `decompressed_size` is only validated as non-negative (by `read_frame`); no upper bound is enforced, so a corrupt footer can request a very large allocation. On a 32-bit target the `source_chunk_data.len() + frame.decompressed_size as usize` addition could overflow — in release builds that wraps, in debug builds it panics.
- The source chunk may legitimately be larger than the LZMA dictionary size; the dictionary size comes from the property bytes, and the preset dictionary is simply the window prefix. `crates/steamkit-tests/tests/lzma.rs` covers both the normal case and the "source chunk larger than the dictionary" case.

#### `crates/steamkit/src/util/win32_helpers.rs` — 79 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/Win32Helpers.cs`
**Purpose:** Supplies the Windows half of the machine-identity fingerprint: the serial number of the physical disk that holds the boot volume. Its only caller is `crate::util::hardware_utils::WindowsMachineInfoProvider::disk_id()`, which turns the serial into the `SteamKit-DiskId` bytes that are sent to Steam.
**How it works:** The C# file is a `partial class` built entirely on P/Invoke. `GetBootDiskNumber()` calls `CreateFile` on `\\.\C:` and then `DeviceIoControl(IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS)` to read `VOLUME_DISK_EXTENTS.Extents[0].DiskNumber`; `GetPhysicalDriveSerialNumber(driveNumber)` calls `CreateFile` on `\\.\PhysicalDriveN` and issues two `DeviceIoControl(IOCTL_STORAGE_QUERY_PROPERTY)` calls — a first one sized for `STORAGE_DESCRIPTOR_HEADER` to learn the descriptor length, a second one for the full `STORAGE_DEVICE_DESCRIPTOR` — and then reads the serial as an ANSI string from `descriptorBuffer + descriptor.SerialNumberOffset`.
The Rust crate forbids `unsafe` (`#![forbid(unsafe_code)]` in `src/lib.rs`), so no P/Invoke is possible. The port obtains the same two facts through PowerShell instead:
- `BOOT_DISK_SERIAL_SCRIPT` takes `$env:SystemDrive`, trims the `:`, runs `Get-Partition -DriveLetter <letter>` to get the partition's `DiskNumber`, then `Get-Disk -Number <n>`, throws if `$disk.SerialNumber` is null, and prints `$disk.SerialNumber.Trim()`.
- `Win32Helpers::get_boot_disk_serial_number() -> Option<String>` runs that script through `run_powershell` and returns `Some(serial)`; on failure it writes `"Failed to get boot disk serial number: {error}"` to `DebugLog` under the category `"Win32Helpers"` and returns `None` — the same "catch the exception and report it" shape as the C#.
- `run_powershell(script) -> Result<String, String>` tries `powershell` first and then `pwsh`, each with `-NoProfile -NonInteractive -Command <script>`, and returns the trimmed stdout. A missing interpreter, a non-zero exit status, or empty stdout makes it fall through to the next candidate; the last error is returned as `Err(String)`.
**Notes and gotchas:**
- The two paths do not have identical access rules: the C# opens `\\.\PhysicalDriveN` with desired access `0` (query device attributes) and asks for `IOCTL_STORAGE_QUERY_PROPERTY`, while the port goes through the `Get-Disk` cmdlet of the Storage module, which reaches the same underlying storage-query APIs. Whether a given token can read the serial therefore depends on the machine's device and PowerShell policy rather than on this code — the observable contract is the same either way: on failure the helper returns `None` and logs the reason.
- Boot-volume discovery differs slightly: the C# derives the volume from the root of `Environment.SystemDirectory` (the drive that holds `C:\Windows\System32`), the Rust script uses `%SystemDrive%`. These agree on a normal system, where `%SystemDrive%` is that same volume.
- The C# explicitly rejects a boot volume that spans more than one disk extent (`extents.NumberOfDiskExtents != 1` throws `InvalidOperationException`). The PowerShell path has no equivalent check: on a spanned or dynamic volume `Get-Partition` can return several rows, `$partition.DiskNumber` becomes an array, the script fails, and the helper returns `None`.
- The serial string itself may differ in formatting between the two implementations. The C# reads the raw `STORAGE_DEVICE_DESCRIPTOR` bytes as an ANSI string; the Rust reads the trimmed `Get-Disk` value. On the same machine the `SteamKit-DiskId` hash computed from the two may therefore not match.
- Spawning a PowerShell process costs tens of milliseconds and happens on every call. There is no caching in either implementation; the helper is only reached when the machine ID is built, so it is not on a hot path.
- `powershell` (Windows PowerShell 5.1) is tried before `pwsh` (PowerShell 7), because `Get-Partition`/`Get-Disk` behave identically in both. Both are resolved through `PATH` by `std::process::Command::new`, so a missing interpreter is reported as `"<program> could not be started: <io error>"`.
- The script sets `$ErrorActionPreference = 'Stop'`, so a failing cmdlet aborts the whole script with a non-zero exit code instead of silently continuing; combined with the empty-stdout check, that is what turns "no serial number" into an error instead of an empty string.
- The module is compiled on every platform (`pub mod win32_helpers;` in `util/mod.rs` is unconditional) even though it only works on Windows — it calls no Windows API directly, it just spawns a shell, so it compiles anywhere but succeeds only where PowerShell and the storage cmdlets exist.

#### `crates/steamkit/src/util/v_zstd_util.rs` — 93 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/VZstdUtil.cs`
**Purpose:** Decodes the "VZstd" container that Valve uses for zstd-compressed depot chunks. The container is a thin wrapper around a standard zstd frame: an 8-byte header, the raw zstd payload, and a 15-byte footer that repeats the CRC32 and the decompressed size.
**How it works:** `VZstdUtil::decompress(buffer: &[u8], destination: &mut [u8], verify_checksum: bool) -> Result<i32, SteamError>`:
- Length guard: `buffer.len() < HEADER_LENGTH (8) + FOOTER_LENGTH (15)` gives `SteamError::Decompression("VZstd data is too short (was N bytes).")`.
- Header magic: `u32::from_ne_bytes(buffer[0..4])` is compared with `VZSTD_HEADER = 0x615A_5356` (the bytes `"VSZa"` in little-endian order); a mismatch gives "Expecting VZstdHeader at start of stream".
- Footer: with `footer = buffer.len() - 15`, `crc32` is read from the header at offset 4, `crc32_footer` from the footer at offset 0 (`buffer[len-15..len-11]`) and `size_decompressed` from the footer at offset 4 (`buffer[len-11..len-7]`).
- Footer magic: the last three bytes must be `'z'`, `'s'`, `'v'`, otherwise "Expecting VZstdFooter at end of stream".
- `destination.len() < size_decompressed` gives `SteamError::InvalidArgument("The destination buffer is smaller than the decompressed data size.")`.
- The payload `buffer[8..footer]` is decompressed with `zstd::stream::decode_all`. The `zstd` crate (version `0.13`, listed in `crates/steamkit/Cargo.toml`) is a safe wrapper over the C zstd library through `zstd-sys`; there is no pure-Rust zstd here. A decompression failure is mapped to `SteamError::Decompression("Failed to decompress Zstd.")`, and a length mismatch to "Failed to decompress Zstd (expected N bytes, got M).".
- The result is copied into `destination`; when `verify_checksum` is set, `crc32fast::hash` over the written bytes is compared with `crc32_footer`. The function returns `Ok(size_decompressed)`.
**Notes and gotchas:**
- Native byte order is deliberate. The C# uses `MemoryMarshal.Read<uint>` / `Read<int>`, which read in native order, and the port mirrors that with `from_ne_bytes`. On a little-endian machine that is identical to little-endian; on a big-endian machine the magic comparison would fail, so the port is faithful to the C# rather than endian-independent. Note that `depot_chunk.rs` itself sniffs the container with the byte string `b"VSZa"`, which *is* endian-independent — only the header inside `VZstdUtil` is not.
- The header CRC and the footer CRC are the same value written twice ("They write CRC32 twice?" in the C#). The port asserts this with `debug_assert_eq!(crc32, crc32_footer)`, matching the C# `Debug.Assert`, which is compiled out of release builds. In debug the Rust port panics where the C# would raise an assert dialog; in release both ignore the mismatch.
- The CRC comparison differs in one respect from the C#. C# compares `Crc32.HashToUInt32(...)` (a `uint`) with `crc32_footer` (an `int`), and C# widens both to `long`, so a negative footer value can never match. The Rust does `crc32_footer as u32`, i.e. it reinterprets the bit pattern, so a negative `int` that holds the correct CRC bits *would* match. The CRC is always small and positive in practice, so the two behave identically on real data.
- The Rust decompresses into a temporary `Vec` and then copies into `destination`, whereas the C# decompresses straight into the caller's array (`ZstdSharp.Decompressor.TryUnwrap`). The Rust path therefore needs one extra allocation of the full decompressed size.
- The footer layout is "crc (4) + decompressed size (4) + unknown (4) + magic `zsv` (3)" = 15 bytes; the "unknown" field is never interpreted by either implementation.
- `verify_checksum` is `false` on the depot-chunk path (`depot_chunk.rs` calls `VZstdUtil::decompress(plaintext, destination, false)`), because the depot chunk verifies its own integrity with the Adler-32 checksum from the chunk header. The VZip and zip paths pass `false` for the same reason; the standalone unit tests in `crates/steamkit-tests/tests/depot_chunk.rs` exercise the checksum path with `true`.
- The zstd frame's own content checksum (if the encoder enabled one) is validated by the zstd library itself, independently of the container CRC.

#### `crates/steamkit/src/util/v_zip_util.rs` — 172 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/VZipUtil.cs`
**Purpose:** Reads and writes Valve's "VZip" container, which is what the Steam client and the LZMA-compressed depot chunks are wrapped in. A VZip stream is a 7-byte header, 5 bytes of LZMA properties, a raw LZMA stream, and a 10-byte footer carrying the CRC and the decompressed size.
**How it works:** The constants are `VZIP_HEADER = 0x5A56` (u16), `VZIP_FOOTER = 0x767A` (u16), `HEADER_LENGTH = 7` (magic + version + timestamp/crc), `FOOTER_LENGTH = 10` (crc + decompressed size + magic) and `VERSION = b'a'`.
- `Frame<'a>` is the port of the C# `internal readonly ref struct VZipUtil.Frame`: `properties: &'a [u8]` (the 5 property bytes), `compressed: &'a [u8]`, `output_crc: u32` and `decompressed_size: i32`. The Rust version borrows from the caller's buffer instead of using `ReadOnlySpan<byte>`.
- `VZipUtil::read_frame(buffer: &[u8], version: u8) -> Result<Frame<'_>, SteamError>` checks the length (`HEADER_LENGTH + LzmaDecoder::PROPERTIES_SIZE (5) + FOOTER_LENGTH`), the header magic, and the version byte; it then reads `output_crc` and `decompressed_size` from the last 10 bytes, validates the footer magic `0x767A`, rejects a negative decompressed size, and returns the two slices. The four bytes at offset 3..7 are deliberately left uninterpreted: they are a creation timestamp for Steam client VZips and a CRC32 for depot chunks.
- `VZipUtil::decompress(buffer, destination, verify_checksum)` calls `read_frame(buffer, VERSION)`, rejects a destination smaller than the declared decompressed size with `SteamError::InvalidArgument`, decodes with `LzmaDecoder::decode(frame.properties, frame.compressed, output, 0)` (no preset dictionary), optionally compares `crc32fast::hash(output)` with `frame.output_crc`, and returns the number of bytes written.
- `VZipUtil::compress(buffer: &[u8]) -> Result<Vec<u8>, SteamError>` is the inverse: it writes the header (magic, version `'a'`, and the CRC of the input in the four "timestamp/crc" bytes), configures the vendored 7-Zip LZMA encoder (`crate::util::lzma::compress::lzma::lzma_encoder::Encoder`) with the same properties as the C# — dictionary size `1 << 23`, 2 pos-state bits, 3 literal-context bits, 0 literal-pos bits, algorithm 2, 128 fast bytes, match finder `"bt4"`, end marker off — writes the 5 property bytes with `IWriteCoderProperties::write_coder_properties`, encodes with `ICoder::code(buffer.to_vec(), &mut out, -1, -1, None)`, and appends the footer (CRC, the input length as a u32, and the footer magic).
**Notes and gotchas:**
- `Frame<'a>` borrows the caller's buffer and therefore cannot outlive it. The C# `ref struct` has the same constraint, but expressed through `ReadOnlySpan<byte>` fields rather than a lifetime parameter.
- The version byte is part of the API: `read_frame` takes it as a parameter so that `VZipDeltaUtil` can parse a `'d'` frame with the same code. `decompress` hard-codes `'a'`, so a delta frame cannot be passed to `decompress` by mistake.
- The header's four ambiguous bytes are ignored entirely, which means the header CRC (which for depot chunks duplicates the footer CRC) is never verified. Only the footer CRC is checked, and only when `verify_checksum` is true.
- `decompress` writes into `&mut destination[..decompressed_size]`, so the tail of the caller's buffer is untouched; the C# does the same through `destination.AsSpan(0, frame.DecompressedSize)`.
- `compress` reserves `buffer.len() + 64` bytes up front and `ICoder::code` takes the input **by value**, so compressing copies the whole input. The C# wraps the input in a `MemoryStream` and does not copy.
- The vendored encoder is a port of the 7-Zip LZMA SDK, so it should produce byte-identical output to the C# `SevenZip.Compression.LZMA.Encoder` for the same properties. The Rust `util/lzma/i_coder.rs` replaces the C#'s `object[]` properties plus runtime type checks with a typed `CoderProperty` enum (`Int32`/`UInt32`/`Boolean`/`String`), and an unsupported type or an out-of-range value still maps to `InvalidParamException` -> `SteamError::InvalidArgument`.
- `EndMarker` is set to `false` in both implementations, so the compressed stream carries no end-of-stream marker; that is precisely why the decoder needs the decompressed size from the footer.
- The C# writes `(uint)buffer.Length` for the decompressed size; the Rust writes `(buffer.len() as u32)`. The behaviour is identical, and both silently truncate for inputs of 4 GiB or more.
- The `verify_checksum` flag defaults to `true` in C# and is an explicit parameter in Rust — every call site passes it, and the depot-chunk path passes `false` because the chunk's own Adler-32 already covers the data.

#### `crates/steamkit/src/util/zip_util.rs` — 196 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/ZipUtil.cs`
**Purpose:** Reads a PKZIP archive that holds exactly one file — the format used by the older, non-VZip depot chunks. The C# hands the archive to `System.IO.Compression.ZipArchive`; the Rust port parses the zip structures itself and inflates the entry with `flate2`, because the crate carries no zip dependency and the part of the format that is needed here is small.
**How it works:**
- `ZipUtil::decompress<R: Read + ?Sized>(stream: &mut R, destination: &mut [u8], verify_checksum: bool) -> Result<i32, SteamError>` reads the whole stream into a `Vec<u8>`, locates the single entry, decompresses it and copies it into `destination`. The C# counterpart takes a `MemoryStream` and returns the same "number of bytes written" `int`.
- `find_end_of_central_directory(data)` scans backwards from the end of the file for the EOCD signature `50 4B 05 06`, limited to the last `0xFFFF + 22` bytes because the record may be followed by a comment of up to 65535 bytes; it returns the offset or `None`.
- `read_first_entry(data)` reads the EOCD: the total entry count at offset 10 (`total_entries`) and the central-directory offset at offset 16. It then reads the 46-byte fixed part of the first central directory file header (signature `50 4B 01 02`) and extracts `compression_method` (offset 10), `crc32` (offset 16), `compressed_size` (offset 20), `uncompressed_size` (offset 24) and `local_header_offset` (offset 42).
- `decompress_entry(data, entry)` seeks to the local file header (signature `50 4B 03 04`, 30-byte fixed part), reads `name_length` (offset 26) and `extra_length` (offset 28), skips them to find the start of the entry data, and slices exactly `compressed_size` bytes from there.
- Method dispatch: `COMPRESSION_STORED (0)` returns the bytes as they are; `COMPRESSION_DEFLATE (8)` inflates them with `flate2::read::DeflateDecoder`; anything else is `SteamError::Decompression("The zip entry uses the unsupported compression method N.")`.
- The inflated result is checked against the declared `uncompressed_size` ("The zip entry is N bytes long, but the archive declares M bytes."), copied into `destination`, and — when `verify_checksum` is set — `crc32fast::hash` of the written bytes is compared with the central directory's CRC ("Checksum validation failed for decompressed file"). The function returns the number of bytes written.
**Notes and gotchas:**
- The "one file" invariant is only *asserted*, not enforced: `DebugLog::assert(entry.total_entries == 1, "ZipUtil", "Expected the zip to contain only one file")` writes `"Assertion Failed! ..."` into the debug log and execution continues, and only the *first* entry is ever decompressed. The C# `DebugLog.Assert` behaves the same way — it is a logging assertion, not a throwing one — so a multi-entry archive silently yields the first entry in both implementations.
- Data descriptors are handled implicitly rather than explicitly. The implementation never reads the sizes out of the local file header: `compressed_size` and `uncompressed_size` always come from the *central directory*, and the general-purpose bit flag is never inspected. That is why an entry written with bit 3 set (sizes unknown in the local header, real sizes in a trailing data descriptor) still works — the central directory carries the authoritative sizes, and the trailing descriptor bytes fall outside the sliced range and are ignored. The C# gets the same behaviour for free from `ZipArchive`.
- The local file header is therefore used only for its signature and for `name_length`/`extra_length`, which is how the implementation finds where the compressed data starts.
- Zip64 is explicitly rejected: if either size equals `u32::MAX`, the port fails with "Zip64 archives are not supported." The C# `ZipArchive` supports Zip64, so a >4 GiB archive works there and fails here. The check only inspects the 32-bit size fields, so a Zip64 entry whose real sizes live in the extra field while the 32-bit fields are not `u32::MAX` would slip past the check and then misparse.
- The EOCD search takes the *last* matching signature while scanning backwards, which is the right choice when a file comment happens to contain those bytes.
- `entry.uncompressed_size as i32` at the end is a narrowing cast, so an entry larger than 2 GiB would produce a negative return value; the number of bytes actually written is `decompressed.len()`.
- Only compression methods 0 and 8 are supported. The C# `ZipArchive` additionally handles bzip2, LZMA and (on modern .NET) deflate64 and zstd entries; those fail here with the "unsupported compression method" error.
- The compressed data is inflated into a fresh `Vec` and only then copied into `destination`, so peak memory is roughly twice the uncompressed size. The C# inflates directly into the caller's array.
- `ZipUtil` is `pub(crate)`. Its only caller is `DepotChunk::process` (`crates/steamkit/src/steam/cdn/depot_chunk.rs`), which recognises the `PK\x03\x04` prefix, wraps the decrypted plaintext in a `Cursor` and calls `ZipUtil::decompress(&mut stream, destination, false)` — checksum verification is off there because the depot chunk checks its own Adler-32.
- The stream is read to the end before anything is parsed, so the whole archive is held in memory twice over (the `Vec` plus the inflated copy). The C# `ZipArchive` over a `MemoryStream` also works on an in-memory buffer, so the memory profile is comparable.

#### LZMA SDK — the compress side

The LZMA **compress** implementation in this crate is a **vendored port of the 7-Zip LZMA SDK**. The C#
tree it was ported from is `csharp-original/SteamKit2/SteamKit2/Util/LZMA/`, and it is itself a vendored
copy of the LZMA SDK (the C# files still carry the
`// <auto-generated /> - This code comes from LZMA SDK` header). The Rust side under
`crates/steamkit/src/util/lzma/` keeps that SDK layout **file for file**: every Rust file maps onto
exactly one C# file, with the same type names, the same member names and the same arithmetic.

#### The compress / decompress split

The SDK's C# tree is split by direction, and so is the Rust tree:

| Direction | C# | Rust |
|---|---|---|
| **Compress** | `Util/LZMA/Compress/**` | `crates/steamkit/src/util/lzma/compress/**` |
| **Decompress** | `Util/LzmaDecoder.cs` (not vendored from the SDK) | `crates/steamkit/src/util/lzma_decoder.rs` |
| Shared API | `Util/LZMA/ICoder.cs` | `crates/steamkit/src/util/lzma/i_coder.rs` |
| Shared helpers | `Util/LZMA/Common/CRC.cs` | `crates/steamkit/src/util/lzma/common/crc.rs` |

Two things follow from that, and they are worth stating up front because they explain most of the
"why is this not where I expected it" questions in this area:

1. **The compress side is the vendored SDK.** `crates/steamkit/src/util/lzma/compress/` is a
   faithful line-by-line port of `Util/LZMA/Compress/`. It is the reference implementation the port was
   written against, and it is the only encoder in the tree — so it has to stay bit-exact with the C#.
2. **The decompress side is *not* the SDK.** `crates/steamkit/src/util/lzma_decoder.rs` is the port of
   `SteamKit2/Util/LzmaDecoder.cs`, which is a **hand-written** decoder in the SteamKit2 C# tree. The
   SDK's own `Decoder/` tree was never vendored into SteamKit2, so there is no `Compress/`-style mirror
   on the decode side. It is a completely separate implementation: single file, own range decoder, own
   bit trees, own probability tables. It is covered by a different section of this guide.

So `lzma/compress/**` and `util/lzma_decoder.rs` do **not** share code, do not share data structures, and
do not even share the same file layout. The only thing they have in common is the wire format: the 5-byte
LZMA properties header followed by a range-coded stream. That is why the round trip
(compress → decompress) is the contract that matters — and why the port's test suite exercises it and
caught the real arithmetic bug documented in the `lzma_encoder.rs` entry below.

The compress side is reached through `Encoder::code` (see the `lzma_encoder.rs` entry below). Its output
is a raw LZMA stream: the 5-byte properties header (`properties byte + dictionary size`) followed by the
range-coded payload. It is *not* the Steam "LZMA" blob format (`LZMA` magic + two length prefixes) — that
framing lives in `crates/steamkit/src/util/lzma_util.rs`, which today only exposes the *decompress*
direction. The compress side's production consumer is `crates/steamkit/src/util/v_zip_util.rs`
(`VZipUtil::compress`), which wraps the raw stream in a VZip frame.

#### Files in this section

| Rust file | Lines | C# counterpart |
|---|---|---|
| `i_coder.rs` | 137 | `SteamKit2/SteamKit2/Util/LZMA/ICoder.cs` |
| `common/crc.rs` | 81 | `SteamKit2/SteamKit2/Util/LZMA/Common/CRC.cs` |
| `compress/range_coder/range_coder.rs` | 139 | `SteamKit2/SteamKit2/Util/LZMA/Compress/RangeCoder/RangeCoder.cs` |
| `compress/range_coder/range_coder_bit.rs` | 95 | `SteamKit2/SteamKit2/Util/LZMA/Compress/RangeCoder/RangeCoderBit.cs` |
| `compress/range_coder/range_coder_bit_tree.rs` | 119 | `SteamKit2/SteamKit2/Util/LZMA/Compress/RangeCoder/RangeCoderBitTree.cs` |
| `compress/lz/i_match_finder.rs` | 44 | `SteamKit2/SteamKit2/Util/LZMA/Compress/LZ/IMatchFinder.cs` |
| `compress/lz/lz_in_window.rs` | 237 | `SteamKit2/SteamKit2/Util/LZMA/Compress/LZ/LzInWindow.cs` |
| `compress/lz/lz_bin_tree.rs` | 529 | `SteamKit2/SteamKit2/Util/LZMA/Compress/LZ/LzBinTree.cs` |
| `compress/lzma/lzma_base.rs` | 113 | `SteamKit2/SteamKit2/Util/LZMA/Compress/LZMA/LzmaBase.cs` |
| `compress/lzma/lzma_encoder.rs` | 1939 | `SteamKit2/SteamKit2/Util/LZMA/Compress/LZMA/LzmaEncoder.cs` |

Line counts are non-blank lines.

---

#### `crates/steamkit/src/util/lzma/i_coder.rs` — 129 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/ICoder.cs`
**Purpose:** The SDK's public interface layer. It declares the traits every coder implements
(`ICoder`, `ISetCoderProperties`, `IWriteCoderProperties`), the `CoderPropID` enum that names the
tunable properties, and the progress callback interface. It contains no algorithm — only the contract
that `LzmaEncoder` fulfils.
**How it works:** `ICoder::code(&mut self, in_stream: Vec<u8>, out_stream: &mut Vec<u8>, in_size: i64,
out_size: i64, progress: Option<&mut dyn ICodeProgress>)` replaces the C#
`Code(Stream inStream, Stream outStream, Int64 inSize, Int64 outSize, ICodeProgress progress)`: the two
`System.IO.Stream`s become an owned `Vec<u8>` for the input and an append-target for the output. `ICodeProgress`
is the C# `ICodeProgress` with `SetProgress` renamed to `set_progress`. `CoderPropID` is a plain Rust enum
whose discriminants match the C# enum's implicit numbering exactly (`DefaultProp = 0`, then
`DictionarySize`, `UsedMemorySize`, `Order`, `BlockSize`, `PosStateBits`, `LitContextBits`, `LitPosBits`,
`NumFastBytes`, `MatchFinder`, `MatchFinderCycles`, `NumPasses`, `Algorithm`, `NumThreads`, `EndMarker`).
`ISetCoderProperties::set_coder_properties(&[CoderPropID], &[CoderProperty])` returns
`Result<(), InvalidParamException>`; `IWriteCoderProperties::write_coder_properties(&mut Vec<u8>)` writes
the 5-byte LZMA properties header.
**Notes and gotchas:** The C# takes `object[] properties` and type-checks every element at runtime with
`is`/`as` casts. The Rust port replaces that with the `CoderProperty` enum (`Int32(i32)`, `UInt32(u32)`,
`Boolean(bool)`, `String(String)`), which turns the runtime check into an exhaustive `match`; the
`as_i32` / `as_bool` / `as_str` helpers are the closest analogue to the C# casts, and `as_i32` on a
`UInt32` truncates by `as i32` (reinterpreting the bits) rather than saturating. `InvalidParamException` is
a `thiserror` error type with the C# message `"Invalid Parameter"`, and it converts into
`SteamError::InvalidArgument`. The C# `ICoder2` interface (the multi-stream variant) is commented out in
the C# source and has **no** Rust counterpart — only `ICoder` was ported. `CoderPropID` carries `Hash` but
not `Ord`; the discriminant values are not written down explicitly except for `DefaultProp = 0`, so the
implicit numbering is the contract and any reordering of the variants would silently change it.

---

#### `crates/steamkit/src/util/lzma/common/crc.rs` — 87 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Common/CRC.cs`
**Purpose:** The SDK's reflected CRC-32 (polynomial `0xEDB88320`, the "IEEE 802.3" / zlib variant). It is
the checksum the 7-Zip archive container stores in its headers and verifies on extraction, and it is the
only CRC in the LZMA SDK — the LZMA stream format itself carries no checksum.
**How it works:** `build_table()` is a `const fn` that reproduces the C# static constructor: for each
byte value `i` it runs the classic reflected table generation, shifting right eight times and XORing the
polynomial `K_POLY = 0xEDB8_8320` whenever the low bit was set. The result is stored as
`pub const TABLE: [u32; 256]`, so the table is materialised at compile time rather than by a runtime
static constructor. `Crc` holds the running `_value`, initialised to `0xFFFF_FFFF` in `Default`/`new`
(mirroring the C# field initializer `uint _value = 0xFFFFFFFF`). `update_byte` folds one byte in as
`TABLE[((value as u8) ^ b) as usize] ^ (value >> 8)` — the same indexing expression as the C#.
`update(data, offset, size)` folds a range in. `get_digest()` returns `value ^ 0xFFFF_FFFF`, the final
XOR. `calculate_digest` / `verify_digest` are the static helpers of the C# class.
**Notes and gotchas:** Two deliberate deviations from the C#, both of them safety improvements that change
no output: (a) the C# `Update(byte[] data, uint offset, uint size)` indexes `data[offset + i]` with no
bounds check and would throw `IndexOutOfRangeException` for an out-of-range range; the Rust `update`
clamps `offset` and `offset + size` to `data.len()` and silently ignores the tail. (b) The C#
`CalculateDigest` deliberately does *not* call `crc.Init()` — it relies on the field initializer — while the
Rust `calculate_digest` calls `Crc::new()`, which is the same thing because `Default` sets
`0xFFFF_FFFF`; the equivalence is intentional and commented in the C#. **The running-checksum half of this
module — the `Crc` struct with `update_byte`, `update`, `get_digest`, `calculate_digest` and
`verify_digest` — is not called by anything in the crate**: in the SDK the CRC belongs to the 7-Zip
*archive* container, not to the LZMA stream, and the Steam-facing containers in this crate
(`util/zip_util.rs`, `util/v_zip_util.rs`, `util/v_zstd_util.rs`) use the `crc32fast` crate instead.
**The table half is used**: `compress/lz/lz_bin_tree.rs` imports `crate::util::lzma::common::crc` and
hashes with `crc::TABLE[byte]`, i.e. the CRC table *is* the match finder's 4-byte hash function. So
`crc.rs` is not dead code — only its streaming API is. Note that the CRC-32 here is the *reflected*
variant with the pre/post-inversion (`0xFFFFFFFF` seed and final XOR), which is the same variant as
`crc32fast` — but in the match finder the table is used purely as a fast byte-mixing table for the hash,
not as a checksum.

---

#### `crates/steamkit/src/util/lzma/compress/range_coder/range_coder.rs` — 153 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/RangeCoder/RangeCoder.cs`
**Purpose:** The binary range **encoder** — the arithmetic core that turns the LZMA symbol stream into
bytes. It maintains a current interval `[low, low + range)` of the number line and, for every symbol,
splits that interval proportionally to the symbol's probability.
**How it works:** The `Encoder` struct holds `low: u64` (the interval's lower bound; the upper 32 bits are
the carry), `range: u32` (the interval size), a one-byte output cache (`cache: u8`, `cache_size: u32`) and
the output buffer itself (`stream: Vec<u8>`, replacing the C# `System.IO.Stream`). `init()` sets
`low = 0`, `range = 0xFFFF_FFFF`, `cache_size = 1`, `cache = 0` and records `start_position` (the length
of the buffer at that moment) so `get_processed_size_add()` can report bytes consumed. The central
routine is `encode(start, size, total)`: it divides the range by `total`, adds `start * range_div` to
`low`, sets `range = range_div * size`, and then renormalises — `while range < TOP_VALUE { range <<= 8;
shift_low(); }` with `TOP_VALUE = 1 << 24`. `shift_low()` is the byte-ejection routine: it writes out the
`cache` byte (plus the carry from the top of `low`), then `0xFF` fill bytes while `cache_size > 1`, then
caches the next byte of `low` and shifts `low` left by 8. Two variants exist for symbols that do not use
the probability model: `encode_direct_bits(v, num_total_bits)` halves the range once per bit, and
`encode_bit(size0, num_total_bits, symbol)` is the generic single-bit split.
**Notes and gotchas:** The C# `Stream` is replaced by an owned `Vec<u8>`; `set_stream`/`release_stream`
become "replace the buffer" / "take the buffer out", `flush_stream` is a no-op (a `Vec` has no buffered
state) and `close_stream` clears the buffer. **Overflow fidelity matters here**: the C# writes
`Low += start * (Range /= total);` where `start * (Range /= total)` is a `uint * uint` multiplication
that wraps at 32 bits *before* being widened to `UInt64`. The Rust reproduces this exactly with
`start.wrapping_mul(range_div) as u64` followed by `low.wrapping_add(...)`. A naive `start * range_div`
in Rust would panic in debug builds, and an `as u64` before the multiply would silently produce a
*different* (non-wrapping) value — so the `wrapping_mul` is load-bearing, not defensive noise.
`range.wrapping_mul(size)` is the same story for the C# `Range *= size`. In `shift_low`, the Rust
`self.cache_size -= 1` would panic on underflow in a debug build, whereas the C# `while (--_cacheSize !=
0)` wraps at zero; in practice `cache_size` is always ≥ 1 when `shift_low` runs, so the two agree.
`get_processed_size_add()` returns `cache_size + (stream.len() - start_position) + 4`; the trailing `+ 4`
is the decoder's "four bytes of look-ahead" allowance from the SDK and is inherited verbatim even though
this encoder never uses it.

---

#### `crates/steamkit/src/util/lzma/compress/range_coder/range_coder_bit.rs` — 109 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/RangeCoder/RangeCoderBit.cs`
**Purpose:** A single binary probability model (`BitEncoder`) plus the static price table that turns
"how likely is this bit" into "how many bits will it cost". Together with the range coder this is the
arithmetic core of LZMA: every literal bit, every match flag and every length digit is one of these.
**How it works:** The model keeps one `prob: u32` in the range `0 .. kBitModelTotal`, where
`NUM_BIT_MODEL_TOTAL_BITS = 11` and `BIT_MODEL_TOTAL = 2048`. `init()` sets `prob = 1024` (an even
odds). `update_model(symbol)` moves the probability toward the observed symbol by 1/32:
`prob += (2048 - prob) >> 5` for a 0-bit, `prob -= prob >> 5` for a 1-bit (`NUM_MOVE_BITS = 5`).
`encode(&mut Encoder, symbol)` performs the range split *and* the update in one go: `new_bound = (range >>
11) * prob`; a 0-bit sets `range = new_bound` and raises `prob`, a 1-bit adds `new_bound` to `low`,
subtracts it from `range` and lowers `prob`; then the usual renormalisation (`if range < TOP_VALUE { range
<<= 8; shift_low(); }`). The pricing side uses a 512-entry static table `PROB_PRICES` built at compile
time by `build_prob_prices()` — the exact transcription of the C# static constructor, with
`kNumBits = kNumBitModelTotalBits - kNumMoveReducingBits = 9` and
`ProbPrices[j] = (i << kNumBitPriceShiftBits) + (((end - j) << kNumBitPriceShiftBits) >> (kNumBits - i - 1))`.
`get_price0()` and `get_price1()` index the table directly; `get_price(symbol)` computes the table index
as `(((prob - symbol) ^ (-(int)symbol)) & (kBitModelTotal - 1)) >> 2`.
**Notes and gotchas:** `get_price`'s index expression is the trickiest line in the SDK and the Rust port
must reproduce it *bit for bit*: in C# `Prob - symbol` is `uint` arithmetic (wraps), and `-(int)symbol`
is a signed negation that the compiler converts back to `uint` (two's complement) before the XOR. The
Rust therefore uses `self.prob.wrapping_sub(symbol) ^ ((symbol as i32).wrapping_neg() as u32)`, masked
with `& (BIT_MODEL_TOTAL - 1)` and shifted down by `NUM_MOVE_REDUCING_BITS = 2`. The two cases collapse
exactly onto the direct accessors: for `symbol == 0` the negation is `0` and the index is `prob`, which
is `get_price0`'s index; for `symbol == 1` the negation is `0xFFFFFFFF`, so the XOR complements the
11-bit value and the index becomes `2048 - prob`, which is `get_price1`'s index. Getting this wrong
does not corrupt the output; it silently changes the encoder's *cost estimates*, which changes which
matches it picks, which changes the compressed bytes — a bug that only shows up as "the round trip
decodes fine but the output is not byte-identical to C#". `PROB_PRICES` is built with a `const fn` using
the literal `6` for `kNumBitPriceShiftBits` rather than the `NUM_BIT_PRICE_SHIFT_BITS` constant that is
also declared on the struct; the two agree (`NUM_BIT_PRICE_SHIFT_BITS = 6`) but the table builder does not
reference it. The table has `1 << (11 - 2) = 512` entries and the shift by `NUM_MOVE_REDUCING_BITS` is
what keeps the index in range.

---

#### `crates/steamkit/src/util/lzma/compress/range_coder/range_coder_bit_tree.rs` — 127 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/RangeCoder/RangeCoderBitTree.cs`
**Purpose:** A tree of `BitEncoder` models used to encode an *n*-bit symbol as a sequence of *n*
context-dependent binary decisions. The tree is how LZMA encodes the distance slot, the length, the
alignment bits and the literal byte.
**How it works:** `BitTreeEncoder` owns `models: Vec<BitEncoder>` of length `1 << num_bit_levels` and the
level count. The tree is a complete binary tree stored in a flat array: node `m` has children `2m` and
`2m+1`, so the model that decides bit *k* of the symbol is indexed by the path taken so far. Node `0` is
never used, and the traversal always starts at `m = 1`. `encode` walks the symbol **most significant bit
first** (`bit_index` from `num_bit_levels-1` down to `0`, `bit = (symbol >> bit_index) & 1`), calling
`models[m].encode(range_encoder, bit)` and descending with `m = (m << 1) | bit`. `reverse_encode` does the
same but **least significant bit first**, which is what the length coder and the literal "reverse" trees
use. `get_price` / `reverse_get_price` are the cost counterparts: they sum the per-bit price along the
same paths without touching the range coder, and are what the encoder uses to decide between candidate
symbols. The two static helpers of the C# — `ReverseGetPrice(BitEncoder[], startIndex, NumBitLevels,
symbol)` and `ReverseEncode(BitEncoder[], startIndex, Encoder, NumBitLevels, symbol)` — become the
associated functions `reverse_get_price_at(models: &[BitEncoder], start_index, num_bit_levels, symbol)`
and `reverse_encode_at(models: &mut [BitEncoder], start_index, range_encoder, num_bit_levels, symbol)`,
which operate on a *slice* of a shared model array instead of a whole tree. That is how the LZMA encoder
reuses one flat `Vec<BitEncoder>` for several trees that must be laid out contiguously.
**Notes and gotchas:** `init()` deliberately starts at index 1 (`self.models.iter_mut().skip(1)`), matching
the C# `for (uint i = 1; i < (1 << NumBitLevels); i++)` — index 0 is dead storage. `reverse_get_price_at`
and `reverse_encode_at` use `start_index.wrapping_add(m)`; the C# writes `Models[startIndex + m]`, which is
unchecked `uint` addition that wraps at 2^32, so the wrapping behaviour is preserved rather than a panic.
Note the deliberate inconsistency between `encode` (`m = (m << 1) | bit`) and `get_price` (`m = (m << 1) +
bit`): it is copied straight from the C#, where `Encode` uses `|` and `GetPrice` uses `+`. The two are
equivalent for `bit ∈ {0, 1}` but the asymmetry is faithful to the original and worth preserving. The
`models` vector is allocated with `vec![BitEncoder::new(); 1 << num_bit_levels]`, so all models start at
probability 0 and *must* be initialised with `init()` before use — an uninitialised model has
`prob = 0`, which makes `encode` compute `new_bound = 0` and silently produces a stream the decoder cannot
read. In the encoder, `init()` is called once per stream.

---

#### `crates/steamkit/src/util/lzma/compress/lz/i_match_finder.rs` — 51 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/LZ/IMatchFinder.cs`
**Purpose:** The two interfaces that describe the LZ77 match finder: `IInWindowStream` (access to the
sliding window) and `IMatchFinder` (the finder itself, which extends `IInWindowStream`). It is pure
interface declaration — no implementation.
**How it works:** `IInWindowStream` declares `set_stream`, `init`, `release_stream`, `get_index_byte(index:
i32)`, `get_match_len(index: i32, distance: u32, limit: u32)` and `get_num_available_bytes()`. `IMatchFinder`
adds `create(history_size, keep_add_buffer_before, match_max_len, keep_add_buffer_after)`, `get_matches(&mut
[u32]) -> u32` and `skip(num: u32)`. The one implementation in this tree is `BinTree` (see
`lz_bin_tree.rs`), which implements both traits by delegating the `IInWindowStream` half to its embedded
`InWindow`.
**Notes and gotchas:** Two type substitutions are forced by the move away from `System.IO.Stream`: the
C# `SetStream(System.IO.Stream)` becomes `set_stream(stream: Vec<u8>)` — the whole input is buffered
up-front rather than streamed — and the C# `Byte GetIndexByte(Int32 index)` keeps its *signed* index
because it may point **before** the current position (that is how the encoder peeks at the byte behind
the cursor). `get_matches` returns the number of values written into the `distances` slice; the pairs it
writes are `(length, distance - 1)`, i.e. the distance is stored zero-based. The C# `Create` throws a bare
`new Exception()` if the history is too large; the Rust trait's `create` returns
`Result<(), SteamError>` instead — the contract is expressed in the type rather than in an exception.
`IMatchFinder` requires `fmt::Debug` as a supertrait bound (the C# has no such notion); this is there so
that a match finder can be stored in a `Box<dyn IMatchFinder>` and still be printable.

---

#### `crates/steamkit/src/util/lzma/compress/lz/lz_in_window.rs` — 264 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/LZ/LzInWindow.cs`
**Purpose:** The sliding window: a fixed-size byte buffer that holds the recent history plus a look-ahead
area, refilled from the input stream as the encoder advances. It is the memory the match finder searches
in and the "dictionary" the compressed stream refers to.
**How it works:** `InWindow` owns `buffer_base: Vec<u8>` (the buffer), `buffer_offset: u32` (where the live
data starts inside the buffer), `pos: u32` (the cursor, relative to `buffer_offset`), `stream_pos: u32`
(how much of the stream has been read into the buffer), `pos_limit: u32` (the offset at which a new
block must be read) and `pointer_to_last_safe_position` (the limit beyond which the buffer must be
slid). `create(keep_size_before, keep_size_after, keep_size_reserv)` sizes the buffer as
`before + after + reserv` and sets `pointer_to_last_safe_position = block_size - keep_size_after`.
`init()` resets the offsets and does the first `read_block()`. `read_block()` computes how much room is
left (`size = (0 - buffer_offset) + block_size - stream_pos`), reads that many bytes from the stream into
the buffer, advances `stream_pos`, and updates `pos_limit = stream_pos - keep_size_after`; when the
stream is exhausted it sets `pos_limit` to the last safe position and raises
`stream_end_was_reached`. `move_pos()` advances the cursor and, when it passes `pos_limit`, slides the
buffer with `move_block()` (which memmoves the live bytes to the front and adjusts `buffer_offset`) and
then refills. `get_index_byte(index)` reads a byte relative to the cursor — negative indices look
*backwards*. `get_match_len(index, distance, limit)` measures how far a match extends, comparing the byte
at `pby + i` with the byte `distance + 1` bytes earlier. `reduce_offsets(sub_value)` shifts every offset
by a constant, which is how the buffer's coordinates are re-based when the window slides.
**Notes and gotchas:** All the offset arithmetic is deliberately `wrapping_*`: the C# uses `UInt32`
arithmetic everywhere, which wraps silently, and the port reproduces that rather than panicking. The
one place this matters most is `init()`'s `ReduceOffsets(-1)`: the C# casts `-1` to `UInt32`
(`0xFFFFFFFF`), so `bufferOffset += 0xFFFFFFFF` (i.e. −1) while `posLimit`, `pos` and `streamPos` all
*increase* by one. The Rust `reduce_offsets(-1)` does `let sub_value = sub_value as u32;` (→ `0xFFFFFFFF`)
and then `wrapping_add`/`wrapping_sub`, which is the same thing. `ByteReader` is a Rust-only
addition: a minimal cursor over a `Vec<u8>` standing in for the C# `System.IO.Stream`. Three places
where the Rust is deliberately *safer* than the C# and therefore not bit-identical in behaviour:
`get_index_byte` returns `0` for an out-of-range or negative absolute position where the C# would throw
`IndexOutOfRangeException`; `get_match_len` treats an out-of-range byte as a mismatch and stops, where
the C# would throw; and `read_block` returns early when the computed `size` is negative or the read would
run past the end of the buffer, where the C# would pass a negative count to `Stream.Read` and throw
`ArgumentOutOfRangeException`. In `move_block`, the C# copies byte-by-byte (with a `// check negative
offset ????` comment the SDK itself left in); the Rust uses `copy_within` and guards the destination
bound.

---

#### `crates/steamkit/src/util/lzma/compress/lz/lz_bin_tree.rs` — 591 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/LZ/LzBinTree.cs`
**Purpose:** The "BT4" / "BT2" match finder. It hashes the bytes at the current position, looks up the
most recent position with the same hash, and walks a binary tree of candidate positions ordered by the
bytes that follow them, keeping the longest matches. This is where LZ77 match discovery happens.
**How it works:** The C# `BinTree` **inherits** `InWindow` and implements `IMatchFinder`; the Rust port
switches to composition: `BinTree` holds a `window: InWindow` field and exposes it through `window()` /
`window_mut()`. The `new` keyword in the C# (`public new void Init()`, `public new void MovePos()`, …)
hides the base members so that `BinTree`'s versions win — the Rust reproduces that by implementing
`IInWindowStream` for `BinTree` with forwarding methods that call `BinTree::init` / `BinTree::move_pos`
rather than the window's. Two flat arrays hold the state: `son: Vec<u32>` (the binary tree nodes, `2 *
cyclic_buffer_size` entries) and `hash: Vec<u32>` (the hash table). `create(history_size,
keep_add_buffer_before, match_max_len, keep_add_buffer_after)` sizes the window, sizes `son` to
`2 * (history_size + 1)`, and computes the hash mask by the classic "round up to a power of two" bit
sweep (`h |= h>>1; h |= h>>2; h |= h>>4; h |= h>>8; h >>= 1; h |= 0xFFFF;`). `set_type(num_hash_bytes)`
picks between the 4-byte hash (`hash_array = true`, `fix_hash_size = HASH2_SIZE + HASH3_SIZE`,
`min_match_check = 4`) and the 2-byte BT2 mode (`hash_array = false`, `num_hash_direct_bytes = 2`,
`min_match_check = 3`). The hash function uses the CRC table from `common/crc.rs`:
`temp = crc::TABLE[b0] ^ b1`, giving `hash2 = temp & 0x3FF`, `hash3 = temp ^ (b2 << 8)` masked to 16
bits, and the main `hash_value = (temp ^ (crc::TABLE[b3] << 5)) & hash_mask`. `get_matches(distances)`
computes the hash, consults the 2-byte and 3-byte hash tables for cheap 2- and 3-byte matches, then walks
the binary tree: `ptr0`/`ptr1` are the two child slots of the current node, `cur_match` walks the
candidate chain, and at each candidate the match length is extended byte by byte. A candidate whose next
byte is *smaller* goes down the `ptr1` side, a *larger* one down `ptr0` — that is the BST ordering. The
walk is bounded by `cut_value` (`16 + match_max_len >> 1` by default, overridable with `set_cut_value`),
which caps how many tree nodes are visited per position. `skip(num)` does the same tree maintenance
without emitting matches. `normalize()` is the periodic re-basing: when `pos` reaches
`MAX_VAL_FOR_NORMALIZE` (`2^31 - 1`), all the stored positions are shifted down by
`pos - cyclic_buffer_size` so the 32-bit position counters do not overflow.
**Notes and gotchas:** The C# `while (++len != lenLimit)` / `count-- == 0` idioms are translated
explicitly rather than naively: the Rust captures `let count_is_zero = count == 0;` *before* the
decrement so it reproduces C# post-decrement semantics exactly (the C# `||` also short-circuits, so
`count` is not decremented when the first condition already breaks the loop — the Rust's early `break`
preserves that). The C# `distances[offset++] = ...` is replaced by the helper `put(distances, offset,
value)`, which **silently ignores out-of-range offsets**. This is a deliberate safety deviation: the C#
would throw `IndexOutOfRangeException`. It also masks a genuine edge case in the C# — the
`if (curMatch3 == curMatch2) offset -= 2;` line can underflow `offset` when only the 3-byte branch
fired, and `UInt32` wraps it to `0xFFFFFFFE`; the Rust reproduces the wrap with `wrapping_sub` but then
the bounds-checked `put` drops the write instead of crashing. `create` returns
`Err(SteamError::InvalidArgument(...))` where the C# throws a bare `new Exception()`. `son` and `hash`
are indexed through `son_get` / `son_set` / `hash_get` / `hash_set` helpers that use `Vec::get` /
`get_mut` and fall back to `0` (i.e. `EMPTY_HASH_VALUE`) — so an out-of-range tree slot behaves like an
empty slot instead of panicking. `BinTree` imports `crate::util::lzma::common::crc` for `crc::TABLE`:
**this is the only place the CRC table is used on the compress side**, and it is what ties
`common/crc.rs` to the encoder. `get_matches` returns the number of `u32` values written; the pairs are
`(length, distance - 1)` and the caller (the LZMA encoder) reads them in that order.

---

#### `crates/steamkit/src/util/lzma/compress/lzma/lzma_base.rs` — 142 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/LZMA/LzmaBase.cs`
**Purpose:** The shared constants and the LZMA state-machine helper that both the encoder and (in the
original SDK) the decoder rely on. It defines the shape of the LZMA format: how many states, how many
repeated distances, how the length coder is split, and how a match length maps to a "position state".
**How it works:** The C# `Base` is an `abstract class` holding `const` members and the nested `State`
struct. The Rust port turns it into a module of free `const`s plus a standalone `State` struct with an
`index: u32`. The constants: `NUM_REP_DISTANCES = 4` (the "rep" distance slots), `NUM_STATES = 12`
(the LZMA state machine), `NUM_POS_SLOT_BITS = 6`, `DIC_LOG_SIZE_MIN = 0`,
`NUM_LEN_TO_POS_STATES_BITS = 2` → `NUM_LEN_TO_POS_STATES = 4` (the "speed optimisation" that picks
which position-slot model a match length uses), `MATCH_MIN_LEN = 2`, `NUM_ALIGN_BITS = 4` →
`ALIGN_TABLE_SIZE = 16` → `ALIGN_MASK = 15`, `START_POS_MODEL_INDEX = 4`, `END_POS_MODEL_INDEX = 14`
→ `NUM_POS_MODELS = 10` (the "special position" models for the end of the distance tree) and
`NUM_FULL_DISTANCES = 1 << (14 / 2) = 128`, the literal/position-state maxima
(`NUM_LIT_POS_STATES_BITS_ENCODING_MAX = 4`, `NUM_LIT_CONTEXT_BITS_MAX = 8`,
`NUM_POS_STATES_BITS_MAX = 4` → `NUM_POS_STATES_MAX = 16`), the length-coder split
(`NUM_LOW_LEN_BITS = 3`, `NUM_MID_LEN_BITS = 3`, `NUM_HIGH_LEN_BITS = 8` →
`NUM_LOW_LEN_SYMBOLS = 8`, `NUM_MID_LEN_SYMBOLS = 8`, `NUM_LEN_SYMBOLS = 272`) and finally
`MATCH_MAX_LEN = MATCH_MIN_LEN + NUM_LEN_SYMBOLS - 1 = 273`. The `State` struct implements the four
transitions of the LZMA state machine: `update_char` (0 → 0, 1..9 → −3, 10..11 → −6), `update_match`
(< 7 → 7, else 10), `update_rep` (< 7 → 8, else 11) and `update_short_rep` (< 7 → 9, else 11);
`is_char_state()` is `index < 7`. `get_len_to_pos_state(len)` subtracts `MATCH_MIN_LEN` and clamps the
result to `NUM_LEN_TO_POS_STATES - 1`.
**Notes and gotchas:** `get_len_to_pos_state` uses `len.wrapping_sub(MATCH_MIN_LEN)`; the C# does a plain
`len -= kMatchMinLen` on a `uint`, which wraps for `len < 2` — the same thing, but the Rust would panic
on a checked subtraction. `State` is `Copy`, so the encoder can keep the state in a local and write it
back cheaply; the C# mutates a `State` struct in place. `NUM_FULL_DISTANCES` is `1 << (END_POS_MODEL_INDEX
/ 2)`, i.e. `1 << 7` = 128 — note that this uses the *integer division* of the constant, not
`NUM_POS_SLOT_BITS`; it is a transcription of `1 << ((int)kEndPosModelIndex / 2)` and both the decoder in
`lzma_decoder.rs` (`FULL_DISTANCE_COUNT = 1 << (END_POS_MODEL_INDEX >> 1)`) and this module agree on 128.
The C# file also carries four commented-out "next state" lookup tables (`kLiteralNextStates` etc.); they
are dead in the C# too and have no Rust counterpart — the Rust computes the transitions with the same
branch logic instead. `MATCH_MAX_LEN = 273` is the ceiling the encoder enforces on a match length; the
`+256` in `NUM_LEN_SYMBOLS` comes from the high-length tree.

---

#### `crates/steamkit/src/util/lzma/compress/lzma/lzma_encoder.rs` — 2130 lines
**C# counterpart:** `SteamKit2/SteamKit2/Util/LZMA/Compress/LZMA/LzmaEncoder.cs`
**Purpose:** The LZMA encoder proper — the optimal-parsing encoder of the 7-Zip SDK. It reads a byte
stream, asks the match finder for candidate matches, runs a dynamic-programming "optimal parser" over
them that prices every candidate with the range coder's cost model, and then emits the chosen parse as
a range-coded LZMA stream. Every probability update, price table and arithmetic detail of the C# is
reproduced, so the produced bytes are identical to the C# encoder's.
**How it works:**

*Types and tables at the top of the file.*

- `EMatchFinderType { BT2, BT4 }` mirrors the C# enum; `Default` is `BT4`.
- `FAST_POS: [u8; 1 << 11]` is the port of the C# `static Byte[] g_FastPos`, built by the `const fn`
  `build_fast_pos()`: `g_FastPos[0] = 0`, `g_FastPos[1] = 1`, then for `slot_fast` in `2..22` it fills
  `1 << ((slot_fast >> 1) - 1)` entries with `slot_fast`. This table maps a *distance* to its **position
  slot** — the LZMA "distance class" `s`, where slot `s` covers the distances
  `[(2 | (s & 1)) << ((s >> 1) - 1), ... + 2^((s>>1)-1))`.
- `get_pos_slot(pos)` (C# `GetPosSlot`) is the standard slot lookup: `FAST_POS[pos]` below `2^11`,
  `FAST_POS[pos >> 10] + 20` below `2^21`, `FAST_POS[pos >> 20] + 40` above.
- `get_pos_slot2(pos)` (C# `GetPosSlot2`) is a *second, shifted* spelling of the same mapping:
  `FAST_POS[pos >> 6] + 12` below `2^17`, `FAST_POS[pos >> 16] + 32` below `2^27`,
  `FAST_POS[pos >> 26] + 52` above. See "Notes and gotchas" for why both exist.

*The coders.*

- `LiteralEncoder2` is one 8-bit literal coder: 0x300 `BitEncoder`s addressed by a binary tree
  (`context` starts at 1 and is walked down with `context = (context << 1) | bit`). `encode` codes a
  plain literal; `encode_matched(match_byte, symbol)` codes a literal that follows a match using the
  "matched" contexts (`state += (1 + match_bit) << 8`, switching back to the plain context as soon as
  the symbol and the match byte diverge); `get_price(match_mode, match_byte, symbol)` is the cost
  counterpart. `LiteralEncoder` holds one `LiteralEncoder2` per (position state, previous byte) pair:
  `create(num_pos_bits, num_prev_bits)` allocates `1 << (num_prev_bits + num_pos_bits)` of them, and
  `get_sub_coder_index(pos, prev_byte)` computes the index as
  `((pos & pos_mask) << num_prev_bits) + (prev_byte >> (8 - num_prev_bits))`.
- `LenEncoder` is the match-length coder: a `choice` and `choice2` bit plus a low tree (3 levels), a
  mid tree (3 levels) per position state, and a single high tree (8 levels). `encode(symbol, pos_state)`
  picks the branch: `symbol < 8` → low; else `choice = 1`, `symbol -= 8`, then `symbol < 8` → mid, else
  → high. `set_prices(pos_state, num_symbols, prices, st)` fills a price slice with the cost of every
  length symbol for that position state.
- `LenPriceTableEncoder` wraps a `LenEncoder` with a cached price table (`prices`, indexed
  `pos_state * NUM_LEN_SYMBOLS + symbol`) and a per-position-state counter (`counters`). After each
  encoded symbol the counter is decremented; when it hits zero the table for that position state is
  rebuilt (`update_table`) — that is how the price estimates stay fresh without rebuilding on every
  symbol.

*The encoder state.*

`Encoder` holds: the LZMA `state: State`, `previous_byte`, the four `rep_distances`; the parser's
`optimum: Vec<Optimal>` (4096 entries, `K_NUM_OPTS = 1 << 12`); the `match_finder: Box<dyn IMatchFinder>`
(a `BinTree` by default, `set_type(4)` = BT4); the `range_encoder: RangeEncoder`; the probability arrays
`is_match` (`NUM_STATES << 4` = 192), `is_rep`, `is_rep_g0`, `is_rep_g1`, `is_rep_g2` (12 each),
`is_rep0_long` (192), the `pos_slot_encoder` (4 trees of 6 levels, one per length-to-position state),
the `pos_encoders` (114 `BitEncoder`s — the "special position" models),
`pos_align_encoder` (a 4-level tree); the two length encoders; the literal encoder; the
`match_distances: Vec<u32>` scratch buffer (`MATCH_MAX_LEN * 2 + 2` entries); the price tables
`pos_slot_prices` (256 entries), `distances_prices` (`NUM_FULL_DISTANCES << 2` = 512) and
`align_prices` (16); and the configuration (`pos_state_bits` = 2, `num_literal_pos_state_bits` = 0,
`num_literal_context_bits` = 3, `dictionary_size` = `1 << 22`, `num_fast_bytes` = `0x20`,
`dist_table_size` = 44).

*The price tables.*

`fill_distances_prices()` rebuilds the distance cost model. It has two halves:

1. For each distance `i` in `START_POS_MODEL_INDEX (4) .. NUM_FULL_DISTANCES (128)` it computes the
   slot, the number of footer bits (`footer_bits = (pos_slot >> 1) - 1`), the base of the slot
   (`base_val = (2 | (pos_slot & 1)) << footer_bits`) and the cost of the "footer" bits:
   `temp_prices[i] = BitTreeEncoder::reverse_get_price_at(&pos_encoders, base_val - pos_slot - 1,
   footer_bits, i - base_val)` (in Rust: `base_val.wrapping_sub(pos_slot).wrapping_sub(1)` — see note 1
   below). This is the cost of coding the low `footer_bits` bits of the distance
   with the shared "special position" bit tree.
2. For each of the four length-to-position states it fills `pos_slot_prices[st + pos_slot]` with the
   cost of coding the slot symbol itself (`pos_slot_encoder[len_to_pos_state].get_price(pos_slot)`), and
   for slots `>= END_POS_MODEL_INDEX (14)` adds the cost of the direct (non-aligned) bits above the
   alignment bits: `extra = (((pos_slot >> 1) - 1) - NUM_ALIGN_BITS) << NUM_BIT_PRICE_SHIFT_BITS`.
   It then assembles `distances_prices`: for `i < 4` the slot price, for `i >= 4`
   `pos_slot_prices[st + get_pos_slot(i)] + temp_prices[i]` — the slot price plus the footer cost.

`fill_align_prices()` fills `align_prices` with the cost of the 4 alignment bits
(`pos_align_encoder.reverse_get_price(i)`).

Both are refreshed during encoding: after every 128 matches (`match_price_count >= 1 << 7`) and every
16 aligned distances (`align_price_count >= ALIGN_TABLE_SIZE`).

*The price helpers used by the parser.*

`get_rep_len1_price` (the cost of a one-byte "short rep"), `get_pure_rep_price` (the cost of using
rep slot `rep_index`), `get_rep_price` (rep cost + length cost) and `get_pos_len_price` (the cost of a
normal match at distance `pos` with length `len`). `get_pos_len_price` is the one that splits:
`pos < NUM_FULL_DISTANCES` → `distances_prices[len_to_pos_state * 128 + pos]`; otherwise →
`pos_slot_prices[(len_to_pos_state << 6) + get_pos_slot2(pos)] + align_prices[pos & ALIGN_MASK]`.

*The optimal parser.*

`get_optimum(position) -> (len, back)` is the heart of the encoder. It is a forward dynamic program
over the `optimum` array:

- If a previous parse is still pending (`optimum_end_index != optimum_current_index`) it simply returns
  the next chunk of it.
- Otherwise it reads the candidate matches (`read_match_distances`), measures the four rep distances
  (`reps` / `rep_lens`), and computes the prices of the four basic options: literal
  (`is_match` price 0 + literal price), match (`is_match` price 1), rep match (`is_match` price 1 +
  `is_rep` price 1), and short rep (`match_byte == current_byte`).
- It then walks forward, at each step trying: a literal (`cur_and1_price`), a short rep, a rep match
  for each of the four rep distances, and a normal match for every candidate distance the match finder
  returned. For each candidate it also tries the "match followed by a literal" continuation
  (`len_test2`), because a match can be followed by a literal that is cheaper than the alternative.
  Every candidate is priced with the *current* probability model and the cheapest one wins; ties keep
  the earlier candidate.
- The `optimum` entries carry the state machine state, the four rep distances (`backs0..backs3`), the
  previous operation kind (`prev1_is_char`, `prev2`, `is_short_rep`) and the price.
- `backward(cur)` walks the chain backwards, rewriting the links so the parse can be emitted
  forwards, and returns the length and the "back" value of the first operation.

The `back` value is the encoder's opcode: `0..4` = "use rep distance 0..3", `>= 4` = "normal match at
distance `back - 4`", and `0xFFFFFFFF` = "literal".

*Emitting the parse.*

`code_one_block()` is the emit loop. For each operation it writes the LZMA symbols:

- **Literal** (`len == 1 && back == 0xFFFFFFFF`): `is_match[complex_state] = 0`, then the literal byte
  through the literal coder — using `encode_matched` when the state is not a char state (i.e. the
  literal follows a match), else plain `encode`. Then `state.update_char()`.
- **Rep match** (`back < 4`): `is_match = 1`, `is_rep = 1`, then the rep-slot bits: `is_rep_g0 = 0`
  and `is_rep0_long` (0 for a one-byte short rep, 1 otherwise) for slot 0; `is_rep_g0 = 1` then
  `is_rep_g1 = 0` for slot 1; `is_rep_g0 = 1`, `is_rep_g1 = 1`, `is_rep_g2 = back - 2` for slots 2 and
  3. A one-byte rep updates the state with `update_short_rep()`; a longer one encodes the length with
  `rep_match_len_encoder` and updates the state with `update_rep()`. Finally the rep-distance array is
  rotated so the used distance becomes `rep_distances[0]`.
- **Normal match** (`back >= 4`): `is_match = 1`, `is_rep = 0`, `state.update_match()`, the length via
  `len_encoder`, then the **distance writer** (see below), then the rep-distance rotation and
  `match_price_count += 1`.

*The distance writer* (there is no separate `write_distance` function in either the Rust or the C# —
this block is inline in `code_one_block` / `CodeOneBlock`):

```
let pos = back - NUM_REP_DISTANCES;          // the real distance - 1
let pos_slot = get_pos_slot(pos);
let len_to_pos_state = get_len_to_pos_state(len);
pos_slot_encoder[len_to_pos_state].encode(pos_slot);
if pos_slot >= START_POS_MODEL_INDEX {
    let footer_bits = (pos_slot >> 1) - 1;
    let base_val = (2 | (pos_slot & 1)) << footer_bits;
    let pos_reduced = pos - base_val;
    if pos_slot < END_POS_MODEL_INDEX {
        BitTreeEncoder::reverse_encode_at(&mut pos_encoders,
            base_val.wrapping_sub(pos_slot).wrapping_sub(1),   // <-- the wrapping index
            &mut range_encoder, footer_bits, pos_reduced);
    } else {
        range_encoder.encode_direct_bits(pos_reduced >> NUM_ALIGN_BITS, footer_bits - NUM_ALIGN_BITS);
        pos_align_encoder.reverse_encode(&mut range_encoder, pos_reduced & ALIGN_MASK);
        align_price_count += 1;
    }
}
```

Distances below slot 4 are coded entirely by the slot; slots 4..13 add the "special position" footer
bits through the shared `pos_encoders` tree; slots 14 and above emit the high bits directly and the
low 4 bits through the alignment tree.

*End of stream, properties, entry point.*

`write_end_marker(pos_state)` (only when `write_end_mark` is set) encodes a synthetic match of length 2
whose distance decodes to `0xFFFFFFFF` — the decoder's end-of-stream marker. It uses slot 63 directly
(not `get_pos_slot`), 30 footer bits and the reduced value `(1 << 30) - 1`, with the top bits written
directly and the low four through the alignment tree. The flag is **off by default** (the C# field
`_writeEndMark` is also false), and the ported test suite explicitly sets `EndMarker` to `false`; the
Steam path always knows the uncompressed size up front and never needs the marker.
`flush(now_pos)` releases the match-finder stream, writes the end marker and
flushes the range coder (`flush_data` = five `shift_low` calls). `write_coder_properties(out_stream)`
writes the 5-byte LZMA header: `properties[0] = (pos_state_bits * 5 + num_literal_pos_state_bits) * 9 +
num_literal_context_bits` and the dictionary size as four little-endian bytes.
`set_coder_properties(prop_ids, properties)` validates and applies the tunables (`NumFastBytes` in
`5..=273`, `DictionarySize` in `1..=2^30` from which `dist_table_size = dic_log_size * 2` is derived,
`PosStateBits` in `0..=4`, `LitPosBits` in `0..=4`, `LitContextBits` in `0..=8`, `MatchFinder` = `"BT2"`
/ `"BT4"`, `EndMarker` = bool; `Algorithm` is accepted and ignored), returning
`Err(InvalidParamException)` for anything else.
`ICoder::code(in_stream, out_stream, in_size, out_size, progress)` is the entry point: it moves the
caller's `out_stream` buffer into the range encoder (so the compressed bytes are *appended* to whatever
was there), runs `set_streams` + the `code_one_block` loop via `run`, then takes the buffer back out and
returns it. `run` loops over `code_one_block` until it reports `finished`, calling
`progress.set_progress(in, out)` between blocks.

**Notes and gotchas:**

*1. The `baseVal - posSlot - 1` wrap-around — a real bug that the port's test suite caught.*

Two places compute `base_val - pos_slot - 1`:

- `fill_distances_prices` → `BitTreeEncoder::reverse_get_price_at(&self.pos_encoders, base_val.wrapping_sub(pos_slot).wrapping_sub(1), footer_bits, i - base_val)`
- the distance writer → `BitTreeEncoder::reverse_encode_at(&mut self.pos_encoders, base_val.wrapping_sub(pos_slot).wrapping_sub(1), &mut self.range_encoder, footer_bits, pos_reduced)`

In the C# this is `UInt32` arithmetic, so for `posSlot == 4` it is `4 - 4 - 1`, which **wraps to
`0xFFFFFFFF`**. The bit tree is then indexed as `Models[startIndex + m]` with `startIndex = 0xFFFFFFFF`
and `m` starting at 1 — so `0xFFFFFFFF + 1` wraps *again*, to `0`, and the tree node actually used is
`pos_encoders[0]`. This is not an accident of the C#: it is how the C# emulates the original C++ SDK's
`bt.ReverseGetPrice(_posEncoders + baseVal - posSlot - 1, ...)`, where the pointer offset of −1 points
one element *before* the array and the tree's first node sits at offset +1. Both languages therefore
end up coding the slot-4 footer bits with the model at index 0.

The Rust port reproduces this with `base_val.wrapping_sub(pos_slot).wrapping_sub(1)` and, inside the
bit tree, `start_index.wrapping_add(m)`.

**This was a genuine bug in the port.** The first version used ordinary (checked) subtraction, which
underflows for `pos_slot == 4` and panics with an arithmetic-overflow error; the ported round-trip test
(compress → decompress) caught it, because the encoder died before it could emit a stream. The fix was
to make the subtraction wrapping *and* the bit tree's index addition wrapping, so the index lands on
`pos_encoders[0]` exactly as the C# does. Do not "clean up" either `wrapping_*` call: the checked version
panics in debug builds, and a naive `start_index + m` would panic in the bit tree as well.

Note the asymmetry that makes this class of bug easy to miss: a wrong *price* index (see the
`range_coder_bit.rs` entry) would **not** panic — it would only make the encoder's cost estimates
slightly wrong, changing which matches the optimal parser picks and therefore the compressed bytes,
while the stream stayed perfectly valid and decodable. The wrapping bug is the loud version of the same
family of mistake.

*2. Two position-slot numberings, and which one is used where.*

`get_pos_slot` and `get_pos_slot2` are two *different spellings of the same slot mapping*, both built on
the `FAST_POS` / `g_FastPos` table:

| | below the first threshold | middle | above |
|---|---|---|---|
| `get_pos_slot` (C# `GetPosSlot`) | `FAST_POS[pos]` (< 2^11) | `FAST_POS[pos >> 10] + 20` (< 2^21) | `FAST_POS[pos >> 20] + 40` |
| `get_pos_slot2` (C# `GetPosSlot2`) | `FAST_POS[pos >> 6] + 12` (< 2^17) | `FAST_POS[pos >> 16] + 32` (< 2^27) | `FAST_POS[pos >> 26] + 52` |

The `+12 / +32 / +52` offsets compensate for the coarser table shift (`>> 6` instead of `>> 10`), so the
two agree on **every distance from 128 up to the 2^30 dictionary limit**. Below 128 they do not agree
(distance 64 gives 12 from `get_pos_slot` and 13 from `get_pos_slot2`; below 64 they are completely
different) — but that range never reaches the `get_pos_slot2` branch, because `get_pos_len_price`
handles `pos < NUM_FULL_DISTANCES` (128) through the `distances_prices` table instead.

The encoder uses **the table form** (`get_pos_slot`) when it *builds the prices* — `fill_distances_prices`
writes `distances_prices[st2 + i] = pos_slot_prices[st + get_pos_slot(i)] + temp_prices[i]` — and when it
*emits* the slot in the distance writer. It uses **the `+12` form** (`get_pos_slot2`) only for the
*price lookup* of large distances inside `get_pos_len_price`. Because the two agree for `pos >= 128`,
the price the parser sees is the price of the slot the encoder will actually write.

The decoder is a third thing entirely and uses **neither**: `crates/steamkit/src/util/lzma_decoder.rs`
decodes the slot straight from the bit tree and reconstructs the distance arithmetically
(`direct_bits = (pos_slot >> 1) - 1`, `distance = (2 | (pos_slot & 1)) << direct_bits`), i.e. it works
in the standard numbering. `g_FastPos` / `FAST_POS` never appears on the decode side.

*3. Distance and offset conventions.*

- The parser's `back` value is an opcode: `0..4` = rep slot, `>= 4` = normal match at distance
  `back - NUM_REP_DISTANCES`, `0xFFFFFFFF` = literal. The match finder stores distances zero-based
  (`distance - 1`), so the value that reaches the distance writer is the *true distance minus one* — the
  decoder adds the one back.
- `additional_offset` compensates for the match finder having been advanced past the current position
  (`read_match_distances` and `move_pos` increment it; the emit loop decrements it by the consumed
  length). Every `get_index_byte(0 - additional_offset)` depends on it; getting it wrong reads the wrong
  byte and silently corrupts the literal context.
- The rep-distance array is rotated after every match (the used distance moves to slot 0), and the
  `optimum` entries carry a snapshot of the four rep distances (`backs0..backs3`) so the parser can
  reconstruct the rep history at any position.
- `K_IFINITY_PRICE = 0x0FFF_FFFF` is the "unreachable" price; the parser marks the not-yet-reachable
  lengths with it before relaxing them.

*4. C# differences.*

- `SetCoderProperties` throws `InvalidParamException` in the C#; the Rust returns
  `Err(InvalidParamException)`. The Rust also rejects a `prop_ids`/`properties` length mismatch, where the
  C# would throw `IndexOutOfRangeException`.
- On a `MatchFinder` change the C# sets `_matchFinder = null` and rebuilds it lazily; the Rust replaces
  the `Box<dyn IMatchFinder>` with a fresh `BinTree` immediately and resets `dictionary_size_prev` so
  that `create()` runs again.
- The C# `ChangePair(smallDist, bigDist)` helper is dead code in the C# (never called) and has no Rust
  counterpart. Likewise the C#'s `Algorithm` property body — the only place that would have set
  `_fastMode` / `_maxMode` — is commented out, as are the three places that would have branched on
  them, so the encoder always runs the full optimal parser on both sides.
- `Encoder::take_output()` is a Rust-only convenience (the C# has no equivalent); the C# writes into the
  `Stream` it was given.
- `Encoder::code` moves the caller's `out_stream` buffer into the range encoder and hands it back
  afterwards, so the compressed bytes are *appended* to whatever the buffer already contained.

*5. Sizes, limits and performance.*

- `optimum` has `K_NUM_OPTS = 4096` entries, so the parser looks ahead at most 4096 bytes
  (`num_available_bytes` is clamped by `K_NUM_OPTS - 1 - cur`). `pos_slot_prices` has 256 entries,
  `distances_prices` 512, `align_prices` 16.
- `fill_distances_prices` is O(128 + 4 * dist_table_size) and is re-run every 128 matches;
  `fill_align_prices` every 16 aligned distances. With the default dictionary (`2^22`) `dist_table_size`
  is 44, so the slot prices cover slots 0..43.
- `MATCH_MAX_LEN` is 273; `num_fast_bytes` defaults to 32 and any match of at least that length is taken
  verbatim without going through the parser.
- The whole input is buffered as a `Vec<u8>` (the C# streams it), and the match finder allocates a
  `son` array of `2 * (dictionary_size + 1)` `u32`s plus a hash table of `hash_mask + 1 + fix_hash_size`
  entries — for the default 4 MiB dictionary that is roughly 32 MiB of `son` plus 8 MiB of hash. This is
  the SDK's design, not something the port introduced, but it is worth knowing before lowering
  `DictionarySize`.
- `get_pos_slot` indexes the 2048-entry `FAST_POS` table with `pos >> 20` for distances ≥ 2^21, so it
  is only valid while `pos < 2^31`. The distance is bounded by the dictionary size, which
  `DictionarySize` caps at `2^30`, so the table is never indexed out of range in practice — but the Rust
  would panic on the index where the C# would throw `IndexOutOfRangeException`.
- The encoder *is* on a production path: `crates/steamkit/src/util/v_zip_util.rs` (`VZipUtil::compress`)
  uses it to build VZip frames, and `crates/steamkit-tests/tests/lzma.rs` exercises it end to end
  (`lzma_util_decompresses_blob` compresses with `Encoder::code` and decompresses with
  `LzmaUtil::try_decompress`, and the VZip delta tests compress against a preset dictionary via
  `set_train_size`). That round trip is the contract the arithmetic notes above protect.

### 5.4 The crate root, the error type, and the CM client


Three files carry the shape of the whole port: the crate root that declares the module tree,
the single error type every fallible API returns, and the `CMClient` that owns the connection to
a Steam CM server. Two more one-liners round the picture out — the samples crate root and the
test crate's placeholder lib target.

The C# tree has no equivalent of the crate root: the C# `SteamKit2` assembly declares its
compile items in `SteamKit2.csproj` and spreads its namespaces across folders. The Rust port
therefore had to *invent* the crate root, and the module tree it declares is a one-to-one mirror
of the C# namespace tree — with two deliberate exceptions, `SteamKit2.Internal` and
`SteamKit2.Base.Generated`, which the C# spreads over several folders and the Rust port
re-groups.

The C# tree also has no single error type: failure is signalled with `System.Exception` and its
subclasses, thrown. The Rust port has exactly one error enum, `SteamError`, and no public API
throws — everything returns `Result<T, SteamError>`. That single change is what makes the
"never panics on runtime-supplied data" rule enforceable: there is nowhere for a panic to hide.

#### `crates/steamkit-tests/src/lib.rs` — 1 line
**C# counterpart:** `SteamKit2/SteamKit2/Tests/Tests.csproj` — the xunit test project. The C# has no library source file at all; `Tests.csproj` is the manifest that names the 35 `*Facts.cs` files.
**Purpose:** Gives the workspace manifest a library target to resolve. The crate carries no library code; the entire test suite lives in the integration-test targets under `crates/steamkit-tests/tests/`.
**How it works:** One comment line and nothing else — no items, no `mod` declarations, no code. Cargo requires a package to have at least one target, and `src/lib.rs` supplies the `steamkit-tests` lib target that keeps the package valid while every real test is an integration test in `tests/`. The 32 test binaries there (`adler32.rs`, `async_job.rs`, `cm_client.rs`, `steam_client.rs`, `smart_cm_server_list.rs`, …) plus `tests/common/mod.rs` and the `tests/data/` binary fixtures are the actual port of the C# `*Facts.cs` files.
**Notes and gotchas:** The C# put 35 facts classes in one xunit project, which is what `Tests.csproj` + the `Files/`, `Helpers/` and `Packets/` subfolders express. Cargo has no equivalent of "several test classes in one project", so the port uses **one integration-test binary per facts file** — a Cargo integration test is its own crate, so this is the closest equivalent, and it is why `src/lib.rs` has to stay empty: a shared helper would have to live in `tests/common/mod.rs` (which is `mod common;`-included by the test binaries) rather than in the lib target. The comment is stale — it says the suite "is being written", but the suite is written: 262 `#[test]` and 65 `#[tokio::test]` functions, 332 passing, 0 failing, 4 ignored. The placeholder text is kept as-is so the file matches the state the port was in when it was created.

#### `crates/steamkit-samples/src/lib.rs` — 6 lines
**C# counterpart:** none directly — the C# `Samples/` directory holds 14 separate console projects (`Samples/000_Authentication/`, `Samples/001_AuthenticationWithQrCode/`, … `Samples/999_TestBed/`) tied together by `Samples.sln`, and no shared library. This file is the Rust substitute for the solution file's shared parts.
**Purpose:** Crate root of the samples crate: a module doc that states the porting rule, and one shared constant.
**How it works:** The `//!` module doc records the contract the crate follows — "Each binary in `src/bin/` is a direct port of the matching C# sample project under `Samples/` in the SteamKit2 repository, keeping the same command-line arguments, console output and flow." One item follows: `pub const VERSION: &str = env!("CARGO_PKG_VERSION");` — a compile-time expansion of the package version from `Cargo.toml`, which inherits `version = "0.1.0"` from `[workspace.package]`. The 14 programs live in `src/bin/*.rs` and are declared **one by one** as `[[bin]]` targets in `crates/steamkit-samples/Cargo.toml` (with explicit `path = "src/bin/000_authentication.rs"` etc.), because Cargo's binary auto-discovery only picks up file names that are valid Rust identifiers and these start with a digit. `src/bin/qr_code/mod.rs` is a shared *module* (not a target) holding the QR-code renderer that `001_authentication_with_qr_code` uses.
**Notes and gotchas:** `VERSION` is currently referenced by **no** sample — it is exported for the samples that print it (the doc comment says "for samples that print it"), but none of the ported programs uses it yet. Because it is `pub` in a lib target it does not trip `dead_code`. The package is `publish = false` and is not a dependency of the library. The samples crate depends on `steamkit-protos` **directly** in addition to `steamkit`: the C# samples use the `SteamKit2.Internal` namespace, which in C# is part of the library assembly itself, whereas in Rust the generated protobuf types live in a separate crate that `steamkit` does not re-export — so the samples need the second dependency and the `Cargo.toml` says so in a comment. The C# had 14 `.csproj` files each with `<OutputType>Exe</OutputType>`; the Rust crate has one package with 14 `[[bin]]` targets, selected with `cargo run --bin 999_test_bed`.

#### `crates/steamkit/src/lib.rs` — 33 lines
**C# counterpart:** `SteamKit2/SteamKit2/SteamKit2.csproj` — the assembly manifest. The C# has no crate-root file: its namespaces are implied by the folders they live in, and its compile items are listed in the `.csproj`.
**Purpose:** The library crate root. It declares the public module tree, forbids `unsafe` for the whole crate, and re-exports the crate's error type at the root.
**How it works:** Three things, in this order.

*Module doc.* The `//!` block carries the C#-namespace → Rust-module mapping as a table: `SteamKit2.Base` → `base`, `SteamKit2.Base.GC` → `base::gc`, `SteamKit2.Base.Generated` → `base::generated` (re-exports of `steamkit_protos`), `SteamKit2.Networking.Steam3` → `networking::steam3`, `SteamKit2.SteamClient` → `steam::steam_client`, `SteamKit2.Internal` → "split across `steam::cm_client`, `steam::handlers` and friends", `SteamKit2.Authentication` → `steam::authentication`, `SteamKit2.CDN` → `steam::cdn`, `SteamKit2.Discovery` → `steam::discovery`, `SteamKit2.WebAPI` → `steam::webapi`, `SteamKit2.Types` → `types`, `SteamKit2.Util` → `util`. The last line of the doc states the intent: "The primary public API is re-exported at the crate root."

*The `unsafe` ban.* `#![forbid(unsafe_code)]` is an inner attribute on the crate root. `forbid` is strictly stronger than `deny`: it cannot be overridden by an `#[allow(unsafe_code)]` anywhere inside the crate, so `unsafe` is a hard compile error rather than a warning. This is the entire safety story of the port — there is no `unsafe` anywhere in `crates/steamkit`, including in the two places where the C# used P/Invoke: `util/mac_helpers.rs` and `util/win32_helpers.rs` re-implement the macOS IOKit and Windows `DeviceIoControl` calls on top of the command-line tools those platforms already ship.

*The module tree.* Six `pub mod` lines — `base`, `error`, `networking`, `steam`, `types`, `util` — and one re-export, `pub use error::SteamError;`, so `steamkit::SteamError` resolves.
**Notes and gotchas:** **There are no crate-level `#![deny(...)]` lints in this file, and no crate-level `async`/tokio setup either.** The lint policy lives in `crates/steamkit/Cargo.toml` instead:

```toml
[lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
```

Cargo lints and inner attributes are independent mechanisms, and the port keeps both: the inner `#![forbid(unsafe_code)]` documents the rule at the crate root, while the Cargo lint applies the same rule to *every* target of the package (including the test targets, which are separate crates and do not inherit the lib target's inner attributes). The second lint is why `CMClient` and `SteamClient` hand-write `impl fmt::Debug`: both hold `Arc<dyn …>` fields — `Arc<dyn ClientMsgDispatcher>`, `Arc<dyn Connection>` — and a trait object is not `Debug`, so a `#[derive(Debug)]` would not compile.

**The library never creates a runtime.** tokio is a plain dependency (`tokio = { version = "1", features = ["full"] }`) and the runtime belongs to the *consumer*: the library uses `tokio::spawn`, `tokio::select!`, `tokio::time::interval`, `tokio::sync::{broadcast, mpsc, oneshot}` and `tokio_util::sync::CancellationToken`, but it never calls `tokio::main` and never builds a `Runtime` itself. Two consequences run through `cm_client.rs`: `CMClient::connect` can only be awaited from inside a runtime, and `CMClient::restart_heart_beat` probes `tokio::runtime::Handle::try_current()` and silently starts no heartbeat when there is none. The C# has the same division of labour in reverse — it has no runtime of its own, but it owns the threads (`Task`, `Timer`, `CancellationTokenSource`).

The module tree is a mirror, not a copy. `SteamKit2.Internal` is *split* across `steam::cm_client`, `steam::handlers`, `steam::cdn`, `steam::discovery`, `steam::webapi` and `steam::authentication`, because in C# that namespace is a flat dumping ground shared by all of them; the reverse mapping is not one-to-one either, since `steam::` holds eight submodules where C# had one namespace. `SteamError` is the **only** root re-export — `SteamResult<T>` is *not* re-exported and lives at `steamkit::error::SteamResult`, the path `SUMMARY.md` §8 names explicitly. The doc table's intra-doc links (`[`base`]`, `[`steam::cm_client`]`, …) only resolve because every one of those modules is `pub`; a private module would turn the link into a rustdoc warning.

#### `crates/steamkit/src/error.rs` — 105 lines
**C# counterpart:** none — there is no single C# counterpart. The C# signals failure with `System.Exception` and its subclasses, of which SteamKit2 defines `SteamKit2.AuthenticationException` (with an `EResult Result` property), `SteamKit2.AsyncJobFailedException`, `SteamKit2.WebAPIRequestException` and `SteamKit2.SteamKitWebRequestException` (an `HttpRequestException`), plus the BCL's `ArgumentException`, `InvalidDataException`, `IOException`, `SocketException`, `TimeoutException` and `OperationCanceledException`. This one enum consolidates all of them.
**Purpose:** The crate-wide error type. Every fallible public API in the port returns `Result<T, SteamError>`; nothing in `crates/steamkit` throws, and nothing panics on data that came off the wire.
**How it works:** Two items, plus three constructor helpers.

*The alias.* `pub type SteamResult<T> = Result<T, SteamError>;` — the alias `SUMMARY.md` §8 requires (`// steamkit::error::SteamResult<T> = Result<T, SteamError> (add this alias in error.rs)`). Every public signature in the crate is written in terms of it.

*The enum.* `#[derive(Debug, thiserror::Error)]` plus `#[non_exhaustive]`, with seventeen variants. `thiserror` reads the `#[error("…")]` attributes to generate the `Display` implementation, and the `#[from]` attributes to generate the `From` conversions and the `source()` chain. The variants:

| Variant | `Display` | Meaning |
|---|---|---|
| `Io(#[from] std::io::Error)` | `i/o error: {0}` | A socket, file or pipe level failure. The `#[from]` makes `?` work on any `std::io::Error`. |
| `Connection(String)` | `connection error: {0}` | The connection to the Steam network could not be established, or was lost. |
| `ConnectionClosed` | `connection closed by peer` | The remote end closed the connection. |
| `Timeout(std::time::Duration)` | `timed out after {0:?}` | The operation did not complete within its deadline. The payload is the **deadline that was exceeded**, not the elapsed time. |
| `JobFailed` | `job failed: the request was rejected by the server` | The server rejected the job — the C# `EMsg.DestJobFailed`, and the counterpart of `AsyncJobFailedException`. |
| `Cancelled` | `operation was cancelled` | The operation was cancelled by the caller (the C# `OperationCanceledException`/`TaskCanceledException`). |
| `Authentication(String)` | `authentication failed: {0}` | Authentication failed — the counterpart of `SteamKit2.AuthenticationException`. |
| `Protocol(String)` | `protocol error: {0}` | The peer sent data that does not match the protocol. |
| `Malformed(String)` | `malformed message: {0}` | A message could not be decoded (the C# `InvalidDataException`). |
| `Decompression(String)` | `decompression failed: {0}` | A compressed payload could not be decompressed. |
| `Crypto(String)` | `cryptographic error: {0}` | A cryptographic operation failed. |
| `InvalidArgument(String)` | `invalid argument: {0}` | A caller supplied an argument the callee cannot work with — the C# `ArgumentException`/`ArgumentNullException`/`ArgumentOutOfRangeException`. |
| `NotFound(String)` | `not found: {0}` | The requested resource does not exist. |
| `Json(#[from] serde_json::Error)` | `json error: {0}` | JSON (de)serialisation failed. |
| `Protobuf(#[from] prost::DecodeError)` | `protobuf decode error: {0}` | A protobuf payload could not be decoded. |
| `ProtobufEncode(#[from] prost::EncodeError)` | `protobuf encode error: {0}` | A protobuf payload could not be encoded. |
| `Other(String)` | `{0}` | Anything that does not fit a more specific variant; the message is rendered verbatim, with no prefix. |

*The conversion helpers.* Three associated functions on the enum, each taking `impl fmt::Display` and returning the matching variant: `SteamError::connection(message)`, `SteamError::protocol(message)` and `SteamError::invalid_argument(message)`. Taking `impl Display` rather than `String` means a `&str`, a `String`, a `std::io::Error` or an `anyhow::Error` can all be passed directly without an intermediate `.to_string()`.
**Notes and gotchas:** **The rule the whole crate is built on is that the library never panics on runtime-supplied data.** A protocol peer can send anything, and the port treats every malformed input as a value, not as a bug. Concretely: `get_packet_msg` returns `Option<Box<dyn IPacketMsg>>` and logs a parse failure instead of unwrapping; `MsgHdrProtoBuf::read` returns `SteamError::Malformed("negative protobuf header length …")` instead of panicking on a negative length; `EMsg::from_value(...).unwrap_or(EMsg::Invalid)` turns an unknown message number into a value rather than an `unwrap`; `decompress_gzip` maps a `flate2` failure to `SteamError::Decompression`; the multi-message unpacker breaks out of its loop on a bad length prefix instead of slicing out of bounds; and `stamp_identity` returns the message bytes unchanged when the layout does not match either header shape. The `#[non_exhaustive]` attribute is the other half of the rule: it means downstream crates must have a `_` arm in a match on `SteamError`, so the port can add variants in a later release without a breaking change.

A few concrete differences from the C# are worth flagging. **`Connection` is not `Io`.** `Io` is a *transport-level* failure (a socket error bubbling up through `#[from]`), while `Connection` is a *state* failure — `cm_client.rs` uses it for "the client is not connected to Steam", "no server candidate is available" and "the connection to Steam has been closed", none of which is an `io::Error`. **`Timeout` carries the deadline**, not the elapsed time, so the message reads "timed out after 5s" rather than "timed out after 30ms". **`JobFailed` is a unit variant**: the C# `AsyncJobFailedException` carries the `EResult` and the job id, but the Rust variant has no payload, so the `eresult` has to be recovered from the message body by the caller. **`Authentication` loses the `EResult`**: the C# `AuthenticationException` has a `Result` property and formats the message as `"{message} with result {result}."`; the Rust variant keeps only the message string, so the result code has to be formatted into the message by the caller. **`Protobuf` and `ProtobufEncode` are two variants, not one**, because `prost` has separate `DecodeError` and `EncodeError` types and `thiserror` allows only one `#[from]` per source type. And `Other` is the escape hatch — `#[error("{0}")]` means the message is rendered with no prefix at all, which is the one place where the error text is not self-describing.

One small documentation artefact: the doc comment on `SteamResult` is a verbatim copy of the one on the enum ("Every fallible public API in this crate returns `Result<T, SteamError>`"), so the alias's rustdoc describes the enum rather than the alias. The same comment is therefore rendered twice in the generated docs.

#### `crates/steamkit/src/steam/cm_client.rs` — 1116 lines

> **Note.** The automatic CM failover and the connection-factory seam described in this
> section were removed for parity; see the porting notes §20. The client now behaves like C#:
> a server-closed session ends and the caller reconnects.
**C# counterpart:** `SteamKit2/SteamKit2/Steam/CMClient.cs` (652 lines, `namespace SteamKit2.Internal`).
**Purpose:** The base client that owns the connection to a Steam CM server. In C# it is an `abstract class CMClient` that `SteamClient` derives from; Rust has no inheritance, so the port makes `CMClient` a concrete struct that `SteamClient` *owns* (`Arc<CMClient>`) and delegates to, with the C# `protected virtual` members expressed as the `ClientMsgDispatcher` trait. It owns the socket, the server list, the session identity and the heartbeat, and it is the only place that turns a received frame into a dispatchable message.
**How it works:** The file is one client struct, one session struct, one trait, one type alias, one factory function and two free functions.

*Constants.* `pub const CURRENT_PROTOCOL: u32 = 65581;` is the protocol version announced in the `ClientHello` — the C# `MsgClientLogon.CurrentProtocol`, and part of the wire protocol, therefore fixed. `const PROTO_MASK: u32 = 0x8000_0000;` is the bit that marks a message as protobuf-backed (the C# `MsgUtil.ProtoMask`). `const EXTENDED_HEADER_SIZE: usize = 36;` with `EXTENDED_HEADER_STEAM_ID_OFFSET: usize = 24` and `EXTENDED_HEADER_SESSION_ID_OFFSET: usize = 32` describe the extended client header: its total size, and where the SteamID and the session id sit inside it.

A server that closes the connection ends the session: the client does not move to another server or repeat the logon, exactly as in C#.

*The events.* The dispatch loop reacts to the connection's `ConnectionEvent`s (`NetMsgReceived(args)`, `Connected`, `Disconnected(args)`), which it reads from the connection's ordered, lossless stream (`ConnectionEvents::events`, see `i_connection.rs`). It used to read three broadcast channels instead, which could drop messages and reorder events (§12 of this guide, the bug history).

*The dispatcher trait.*

```rust
#[async_trait]
pub trait ClientMsgDispatcher: Send + Sync {
    async fn on_client_msg_received(&self, client: &CMClient, packet: &dyn IPacketMsg) -> bool;
    async fn on_client_connected(&self, client: &CMClient);
    async fn on_client_disconnected(&self, client: &CMClient, user_initiated: bool);
}
```

This is the Rust equivalent of the three `protected virtual` members of the C# class (`OnClientMsgReceived`, `OnClientConnected`, `OnClientDisconnected`). Returning `false` from `on_client_msg_received` aborts the connection. The only implementation in the tree is `SteamClientDispatcher` in `steam/steam_client/steam_client.rs`; it holds a `Weak<SteamClient>` and upgrades it per call, so the dispatcher never keeps the client alive and no reference cycle is created.

*The connection factory seam.*

```rust
pub type ConnectionFactory = Arc<
    dyn Fn(&CMClient, &ServerRecord, ProtocolTypes) -> Result<Arc<dyn Connection>, SteamError>
        + Send
        + Sync,
>;
```

The C# has no such seam: `CMClient.CreateConnection` is a private method that hard-codes the three
transports. An earlier revision of the port exposed a `ConnectionFactory` injection point; it was
removed for parity (the porting notes §20.2). The default factory is still a function so the steam core
never has to name the concrete transport classes: the networking module owns `TcpConnection`,
`UdpConnection` and `WebSocketConnection`, and the steam core only sees `Arc<dyn Connection>`.

*`default_connection_factory` — the C# `CMClient.CreateConnection`.*

The C# tries the transports in the order **WebSocket → TCP → UDP**:

```csharp
IConnection CreateConnection( ProtocolTypes protocol )
{
    if ( protocol.HasFlagsFast( ProtocolTypes.WebSocket ) )
        return new WebSocketConnection( this, Configuration.HttpClientFactory( HttpClientPurpose.CMWebSocket ) );
    else if ( protocol.HasFlagsFast( ProtocolTypes.Tcp ) )
        return new EnvelopeEncryptedConnection( new TcpConnection( this ), Universe, this, DebugNetworkListener );
    else if ( protocol.HasFlagsFast( ProtocolTypes.Udp ) )
        return new EnvelopeEncryptedConnection( new UdpConnection( this ), Universe, this, DebugNetworkListener );

    throw new ArgumentOutOfRangeException( nameof( protocol ), protocol, "Protocol bitmask has no supported protocols set." );
}
```

The Rust port tries them in the same order as the C#: **WebSocket, TCP, UDP**.

1. **WebSocket first.** `WebSocketConnection::new_tungstenite_for_host(record.get_host())`. It is returned **directly**, *not* wrapped in `EnvelopeEncryptedConnection` - the CM websocket transport carries plain Steam frames, exactly as in the C#.
2. **TCP.** `EnvelopeEncryptedConnection::with_debug_network_listener(Arc::new(TcpConnection::new()), universe, debug_network_listener)`.
3. **UDP.** The same as TCP, over `UdpConnection::new()`.
4. **Nothing matched** → `Err(SteamError::InvalidArgument("Protocol bitmask has no supported protocols set.".to_owned()))` — the C# throws `ArgumentOutOfRangeException` with the same sentence.

`universe` and `debug_network_listener` are read from the `&CMClient` the factory is handed, which is why the first parameter is the client and not just the record.

*Why the factory takes the `&ServerRecord`.* `SUMMARY.md` §8 freezes `IConnection::connect` to `async fn connect(&self, end_point: SocketAddr, timeout: Duration)`. A `SocketAddr` is an address — it has **no host name**. The C# has no such restriction: it builds the websocket URI from the `EndPoint`, and a `DnsEndPoint` keeps the host name (`cmp1-sea1.steamserver.net`), which is also the **TLS server name** — the name the `wss://` handshake presents as SNI and the name Steam's certificate is issued for. Resolving the name to an address and handing only the address to the transport would make the TLS handshake present an IP address, and the certificate would not validate. The port therefore keeps the name alive: `ServerRecord::get_host()` returns the host of the record's `EndPoint` (an `EndPoint::Dns` keeps it, an `EndPoint::Ip` has none), `WebSocketConnection::new_tungstenite_for_host` stores it, and `WebSocketConnection::connect` then calls `construct_uri_for_host(host, end_point.port())` instead of `construct_uri(end_point)`. The *record* is the natural carrier of that name, which is exactly why the factory signature takes it.

*The bug this fixed — the port could not reach Steam at all.* The port shipped a `WebSocketConnection` and a `tokio-tungstenite` factory, but `default_connection_factory` **never used them**, and its order was TCP → UDP → WebSocket instead of the C#'s WebSocket → TCP → UDP. A WebSocket-only server candidate therefore could not connect at all: the protocol mask would contain only the `WEB_SOCKET` bit, the factory fell through to the UDP branch, and it built a `UdpConnection` — the Steam datagram protocol — to talk to a TLS port. That matters because the **default fallback server list holds the WebSocket-only server first**: `SmartCMServerList::resolve_server_list` ends with

```rust
let default_servers = vec![
    ServerRecord::create_websocket_server(&Self::default_server_websocket()).await?,
    ServerRecord::create_dns_socket_server(&Self::default_server_netfilter()).await?,
];
```

— `cmp1-sea1.steamserver.net:443` (WebSocket only) ahead of `ext1-sea1.steamserver.net:27017` (TCP/UDP). With the old factory, the first candidate in the default list was exactly the one that could never be connected to. The default configuration's mask is `ProtocolTypes::TCP | ProtocolTypes::WEB_SOCKET` (the C# default is the same), so `record.protocol_types() & configuration.protocol_types()` on the WebSocket record yields `WEB_SOCKET` alone, and the broken branch was taken every time.

The bug was found by *running a sample*, not by reading the code: `999_test_bed` could not get past the handshake. The fix (§12, `steam/cm_client.rs`) was to wire the websocket transport up and to plumb the host name through — `WebSocketConnection::new_tungstenite_for_host`, so the websocket URI carries the TLS server name. With it, the port reached a real Steam CM server over the websocket and exchanged a correct `ClientHello` (`protocol_version = 65581`) and `ClientLogon` (§12, the live CM server exchange).

**Where the port stands today on the two transports.** The later, controlled comparison — the C# reference and the Rust port capturing the same traffic through the same `DebugNetworkListener` mechanism — shows that **the TCP transport completes the whole CM flow**: the channel-encryption handshake, then `ClientHello`, then `ClientLogon` (269 bytes, the same size the C# reference sends), and the server answers with a `ClientLogOnResponse` (33 bytes). Over the websocket the logon goes out and the server answers it too. Two defects had
hidden that: `WebSocketConnection::local_ip` returned `None` where the C# returns
`IPAddress.None` (which is `255.255.255.255`, not null), so the logon was refused before it was
sent; and a test of ours logged on before the `Connected` event had arrived, which the
server refuses. Both are fixed. What is left is that the server closes a websocket CM
session after a minute or two while a TCP session stays up, so
`default_connection_factory` prefers the transport that holds the connection (§15 of this guide, the porting notes §13).

One deviation survives all of this and is still open: the `CMWebSocket` HTTP-client purpose does not reach the transport. The C# drives a `ClientWebSocket` through an `HttpMessageInvoker`, so the user's HTTP client — its proxy, its TLS configuration, its headers — applies to the CM websocket; the Rust port opens the socket with `tokio_tungstenite::connect_async`, which performs its own handshake, so `HttpClientPurpose::CMWebSocket` exists but is unused by the default factory.

*The `CMClient` struct and its state.* Twenty-five fields:

*Configuration and identity.* `configuration: Arc<SteamConfiguration>`, `id: String` (the per-instance identifier), `servers: Arc<SmartCMServerList>` (the bootstrap list, cloned from the configuration), `dispatcher: Arc<dyn ClientMsgDispatcher>`.

*The connection and its plumbing.* `connection_factory: RwLock<Option<ConnectionFactory>>`, `connection: RwLock<Option<Arc<dyn Connection>>>`, `connection_cancellation: RwLock<Option<CancellationToken>>` (cancels the dispatch loop), `outbound: RwLock<Option<mpsc::UnboundedSender<Bytes>>>` (the queue of frames waiting to be written).

*Flags.* `expect_disconnection: AtomicBool` (the C# `ExpectDisconnection` — the caller expects the server to close the connection, so the teardown must not blame the server), `is_connected: AtomicBool`, `connected_handled: AtomicBool` (guards against a duplicate `Connected` event).



*Session ownership.* `generation: AtomicU64` is incremented by every `connect`; each dispatch loop serves one generation, and installs or removes the client's connection only while its generation is current, so a superseded loop can never touch a newer session. `announced_generation: AtomicU64` is the latest generation whose disconnection has been announced; `claim_disconnect_announcement(generation)` is a `fetch_max` that returns `true` exactly once per generation, which is what lets the dispatch loop and a concurrent `disconnect` race without announcing a disconnection twice.

*Session state.* `session_token: AtomicU64` (the AM session token), `cell_id: RwLock<Option<u32>>`, `session_id: RwLock<Option<i32>>`, `steam_id: RwLock<Option<SteamID>>`, `public_ip: RwLock<Option<IpAddr>>`, `ip_country_code: RwLock<Option<String>>`, `user_country_code: RwLock<Option<String>>`.

*Diagnostics and the heartbeat.* `debug_network_listener: RwLock<Option<Arc<dyn IDebugNetworkListener>>>`, `heart_beat: RwLock<Option<HeartBeat>>`.

Every `RwLock` here is `parking_lot::RwLock`, **not** tokio's — the guards are held across no `.await` points, so a synchronous lock is both correct and cheaper. The `Atomic*` fields cover the handful of values that are read on the hot path (`is_connected`, `session_token`).

`struct HeartBeat { cancel: CancellationToken, task: tokio::task::JoinHandle<()> }` is the handle to the background heartbeat: the token stops it, the handle aborts it.

`impl fmt::Debug for CMClient` is hand-written and prints only `id`, `is_connected`, `session_id`, `steam_id` and `cell_id` — the `missing_debug_implementations = "warn"` lint demands a `Debug` impl, and the struct's `Arc<dyn …>` fields make a derive impossible.

*The accessors.* `configuration()`, `id()`, `servers()`, `local_ip()`, `current_end_point()`, `public_ip()`, `ip_country_code()`, `user_country_code()`, `universe()`, `is_connected()`, `session_token()`, `cell_id()`, `session_id()`, `steam_id()`, `connection_timeout()`, `debug_network_listener()`, `set_debug_network_listener()`, `expect_disconnection()`, `set_expect_disconnection()` and `log_debug(category, message)`. `log_debug` does exactly what the C#'s `CMClient.LogDebug` does: it builds the **full category** `"{id}/{category}"` — the C#'s `string.Concat( ID, "/", category )` — writes the line to the global `DebugLog` sink (`DebugLog::write_line(&full_category, message)`), and *in addition* emits a `tracing::debug!` event carrying the client id and the full category as structured fields. The port used to log through `tracing` only, which meant a `DebugLog` listener never saw the line; that is fixed, and the two `debug_log.rs` facts now assert it (see 7.2.7). `log_debug_args(category, message, args)` is the C# `params object[]` form; `current_end_point()` returns `Option<EndPoint>` reconstructed from the server record.

*`connect()` — the server candidate, the protocol mask, the connection.*

```rust
pub async fn connect(self: &Arc<Self>, cm_server: Option<ServerRecord>) -> Result<(), SteamError>
```

The receiver is `self: &Arc<Self>` because the method spawns the dispatch task, which needs to hold an owned handle to the client.

1. `self.disconnect_with(true).await` — the C# `Connect` opens with `Disconnect( userInitiated: true )`, so a reconnect never leaves the old socket alive. `connect` itself then increments `generation`, resets `expect_disconnection` and `connected_handled` to false and installs a fresh `CancellationToken` in `connection_cancellation`.
2. **Server candidate.** `match cm_server { Some(record) => Some(record), None => self.servers.get_next_server_candidate_async(self.configuration.protocol_types()).await }` — the C# `Servers.GetNextServerCandidateAsync( Configuration.ProtocolTypes )`, the same call with the same argument.
3. **No candidate.** The client logs "Server record task returned no result.", calls `on_client_disconnected(false)` and returns `Err(SteamError::Connection("no server candidate is available".to_owned()))`. The C# logs the same sentence and calls `OnClientDisconnected( userInitiated: false )` from inside its continuation; it has no error to hand back, because its `Connect` returns `void`.
4. **The endpoint.** `let end_point = record.end_point();` — the C# passes `record.EndPoint` to `newConnection.Connect(...)` and resolves the DNS name at that moment. The Rust port resolves the host name *when the server record is created* (`SUMMARY.md` §8 requires a `SocketAddr`), so `end_point()` is already an address; the host name survives in the record's `EndPoint::Dns` and is used by the websocket transport.
5. **The session is opened.** `self.open_session(record, generation).await` — everything that creates a connection now lives in `open_session`; `connect` is `open_session` plus starting the dispatch loop.
6. **The dispatch loop is spawned** with the session **by value**: `tokio::spawn(async move { this.dispatch_loop(session, task_cancellation, generation).await; })`.
7. **The socket is opened last:** `connect_target.connect(end_point, self.connection_timeout()).await`. On failure the client logs "Unhandled exception when attempting to connect to Steam: {error}", cancels the dispatch loop, calls `end_session(generation, ..., false)` — which announces the disconnection once — and returns the error. Cancelling the loop matters: it used to be left running, and the WebSocket transport, which also raises `Disconnected` for a failed attempt, could make it announce the failure a second time or start a move to another server. The C# does the same from the second continuation of its `Task.ContinueWith`, but it has no error to hand back — its `Connect` is `void` and the failure surfaces as a `DisconnectedCallback`.
8. `Ok(())`.

*The session.* The connection and its event stream travel together in a private struct:

```rust
struct ConnectionSession {
    connection: Arc<dyn Connection>,
    events: mpsc::UnboundedReceiver<ConnectionEvent>,
}
```

The dispatch loop **owns** the session. It owns the session by value, so it can hand it back when the connection ends.

*`open_session()` — everything `connect` used to do.*

```rust
async fn open_session(self: &Arc<Self>, record: ServerRecord, generation: u64) -> Result<ConnectionSession, SteamError>
```

1. **The protocol mask.** `let protocol = ProtocolTypes(record.protocol_types().raw() & self.configuration.protocol_types().raw());` — the C# `record.ProtocolTypes & Configuration.ProtocolTypes`. The intersection is what the candidate supports *and* what the client is configured for. `ProtocolTypes` is a newtype over the raw `u32` bit mask (`SUMMARY.md` §8), so the `&` is done on `.raw()` and the result re-wrapped.
2. **The connection.** `let factory = self.connection_factory.read().clone(); let connection = match factory { Some(factory) => factory(self, &record, protocol)?, None => default_connection_factory(self, &record, protocol)? };`. An installed factory wins over the default one.
3. **The event stream is subscribed before the socket is opened** — `connection.events()`, so that no event can be missed. The C# does the same thing with `newConnection.NetMsgReceived += NetMsgReceived;` before `newConnection.Connect(...)`.
4. **The connection and the outbound queue are installed** — under the connection lock, and only while `generation` is still current; a superseded session gets `SteamError::Cancelled` instead of overwriting the newer session's connection. The outbound queue is `mpsc::unbounded_channel::<Bytes>()`: the sender goes into `self.outbound`, and a spawned task drains the receiver and calls `writer_connection.send(bytes).await`. A write failure is logged at debug level and the loop continues. The queue exists so that the *order* in which messages were handed to `CMClient::send` is preserved even though the write itself is asynchronous.
5. The `ConnectionSession` is returned.

*The cycle this breaks.* `connect` starts the dispatch loop, and the dispatch loop starts a connection — it has to, because the loop is what asks for the next server when the current one asks to be replaced. Neither of the two can therefore be a plain `async fn` that calls the other: the compiler would need the size of each future, and each future contains the other. Splitting the work into `open_session` (which only creates a connection) and `connect` (which creates a session and starts the loop) is what breaks the cycle;

*There is no server switch.* The client does not move to another CM server and does not repeat a logon: a connection that ends ends the session, and the caller reconnects, exactly as in C#.

*Disconnecting.* `disconnect()` is `disconnect_with(true)`. `disconnect_with(user_initiated)` stops the heartbeat, cancels the `connection_cancellation` token (which ends the dispatch loop before it can act on the `Disconnected` event this raises), drops the outbound sender (which ends the writer task, because the channel closes), takes the connection out of its slot, calls `connection.disconnect(user_initiated)`, and — if it wins `claim_disconnect_announcement` for the current generation — `teardown(connection, user_initiated)`. The announcement does not depend on the connection being in its slot: during a server switch it is not, and the session is still live. The C# `Disconnect(bool)` additionally waits for the connection-setup task to complete before it touches the connection, so that a `Connect` still resolving a server record cannot race the teardown.

*Sending.* `pub fn send(&self, msg: &dyn IClientMsg) -> Result<(), SteamError>` is **synchronous**: it only serialises the message and pushes it onto the outbound queue; the actual write happens in the writer task. It logs the message — `CM: --> {:?}` with the message type — notifies the debug listener (`on_outgoing_network_message(msg.msg_type(), &serialized)`) and then pushes the bytes. With no outbound channel it returns `Err(SteamError::Connection("the client is not connected to Steam".to_owned()))`; with a closed channel, `Err(SteamError::Connection("the connection to Steam has been closed".to_owned()))`.



The C# `Send` differs in three ways. It *mutates* the message it is handed (`msg.SessionID = sessionID.Value; msg.SteamID = steamID;`), it swallows `IOException` and `SocketException` — "we'll swallow any network failures here because they will be thrown later on the network thread, and that will lead to a disconnect callback down the line" — and it has no equivalent of the per-message log line. The Rust port applies the identity to the **serialised bytes** instead (the `send` signature is frozen by `SUMMARY.md` §8), reports the failure instead of swallowing it, and logs every message as the C# does.

*Stamping the identity.* `serialize_message` reads the current `session_id` and `steam_id`; if neither is known the message is serialised unchanged — the C# does the same, because it only sets the fields it has. Otherwise `stamp_identity(msg, bytes, session_id, steam_id)` rewrites the bytes:

* *Protobuf layout* — `msg` (4 bytes) ‖ header length (4 bytes) ‖ `CMsgProtoBufHeader` ‖ body. The port decodes the protobuf header, sets `client_sessionid` and `steamid`, re-encodes it and rewrites the length. Every step is bounds-checked: `bytes.len() < 8`, a negative header length, and `bytes.len() < 8 + header_length` all return the bytes unchanged rather than panicking.
* *Extended layout* - if `msg.is_extended()` **and** `bytes.len() >= 36` and `bytes[4] == 36` (the `header_size` byte, which is what distinguishes an extended header from a plain one), the SteamID is written at offset 24 and the session id at offset 32, both little-endian.
* *Plain layout* — a 20-byte `MsgHdr` message carries neither, and is returned unchanged.

*The frame parser.* `get_packet_msg(&self, data: Vec<u8>) -> Option<Box<dyn IPacketMsg>>` is the Rust counterpart of the C# `static IPacketMsg? GetPacketMsg( byte[] data, ILogContext log )`.

* Fewer than four bytes and the frame cannot even hold a message number: the client logs "PacketMsg too small to contain a message, was only N bytes. Message: 0x…" and returns `None`.
* `raw = u32::from_le_bytes(data[0..4])` and `e_msg = EMsg::from_value((raw & !PROTO_MASK) as i32).unwrap_or(EMsg::Invalid)` — the C# `MsgUtil.GetMsg(rawEMsg)`, which strips the protobuf flag off the high bit. An unknown message number becomes `EMsg::Invalid` instead of throwing.
* `ChannelEncryptRequest`, `ChannelEncryptResponse` and `ChannelEncryptResult` are **always** parsed as `PacketMsg` (the plain 20-byte `MsgHdr`), even when the protobuf bit is set — the C# has the identical special case, commented "certain message types are always MsgHdr". These three are the channel-encryption handshake, which runs before the connection is encrypted and therefore before the protobuf framing is in play.
* Otherwise the protobuf bit decides: `raw & PROTO_MASK != 0` → `PacketClientMsgProtobuf` (header `MsgHdrProtoBuf`), else `PacketClientMsg` (header `ExtendedClientMsgHdr`).
* A parse failure is logged — "Exception deserializing emsg N (isProto): E" — and `None` is returned. The frame is dropped; the caller decides what to do with it.

*The three header types.* They are defined in `crates/steamkit/src/base/generated/steam_language_internal.rs`, and the distinction matters because the three layouts put the message number in different places.

* `MsgHdrProtoBuf` — the protobuf header. Wire layout: the **protobuf-flagged** message number (4 bytes) ‖ the length of the protobuf header (4 bytes) ‖ that many bytes of `CMsgProtoBufHeader`. The `CMsgProtoBufHeader` itself (Valve's `steammessages_base.proto`) carries `steamid`, `client_sessionid`, `jobid_source`, `jobid_target`, `eresult`, `error_message`, `messageid` and a long tail of routing fields — but **not** the message number. The message number is the 4-byte value in front of it, and `MsgHdrProtoBuf` *owns* it: the type's first field is `msg: EMsg`, and its `read` implementation re-reads that same 4-byte value (`get_msg(raw as u32)`) when the header is deserialised. So the protobuf header type carries the message number itself, and the header body follows it.
* `MsgHdr` — the plain header, 20 bytes: `msg` (4) ‖ `target_job_id` (8) ‖ `source_job_id` (8). Fixed size, no identity fields.
* `ExtendedClientMsgHdr` - 36 bytes: `msg` (4) ? `header_size` (1) ? `header_version` (2) ? `target_job_id` (8) ? `source_job_id` (8) ? `header_canary` (1) ? `steam_id` (8) ? `session_id` (4). Here the message number is the first field of a fixed-size struct and the remaining fields follow it; the `header_size` byte at offset 4 is what tells an extended header apart from a plain one, and it is the byte `stamp_identity` tests against `EXTENDED_HEADER_SIZE`, together with `IClientMsg::is_extended()`.

The practical consequence of "the header owns the message number" is that a parser must hand the header the **whole frame, starting at byte 0**. The nethook analyzer used to strip the 4-byte message number first and *then* let the header read it again, so it read the header length out of the body and failed with "attempted to read 75541768 bytes at offset 8" (§12, `steamkit-tools/nethook/message_reading.rs`). The port's `PacketClientMsgProtobuf::new` hands the full `data` to `MsgHdrProtoBuf::read`, which reads the number, the length and the header in one pass.

*The dispatch loop.* `dispatch_loop(self: Arc<Self>, mut session: ConnectionSession, cancellation: CancellationToken, generation: u64)` **owns the session** and is a `biased` `tokio::select!` over two sources: the cancellation token, which ends the loop at once (whoever cancelled it has announced the disconnection), and the session's ordered event stream. The stream is the current session's. Because it is one ordered, unbounded stream, every message is delivered, and the `Connected` and `Disconnected` events arrive exactly where the connection raised them relative to the messages.



Then:

* `NetMsgReceived(args)` → `get_packet_msg(args.data)`; a `None` means the frame could not be parsed, so the client logs "Packet message failed to parse, shutting down connection", calls `disconnect_with(false)` and breaks the loop. Otherwise the packet is dispatched. **The incoming message is logged first** — `CM: <-- {:?}` with the message type — exactly as the C# logs every CM message it receives.
* `Connected` → `handle_connected(&session.connection)`.
* `Disconnected(args)` -> `end_session(generation, ..., user_initiated)` announces the disconnection and the loop ends. The session is not resumed on another server; that is the caller's decision, as in C#.
* The stream ends → `end_session(generation, ..., false)`, and the loop ends.

`end_session(generation, connection, user_initiated)` is the C#'s `Disconnected` handler: while `generation` is current it removes the client's connection and the outbound queue, and if it wins `claim_disconnect_announcement` it calls `teardown`. A loop that a later `connect` has superseded does nothing.

*The dispatch pipeline.* `dispatch_client_msg(&Arc<Self>, packet: Option<&dyn IPacketMsg>) -> bool` runs the base handling **first** and only then hands the message to the concrete client: `on_client_msg_received(packet).await` (the base) and then `self.dispatcher.on_client_msg_received(self, packet).await` (the client). This mirrors the C# `base.OnClientMsgReceived( packetMsg )` that `SteamClient.OnClientMsgReceived` starts with. A `false` from either stage aborts the connection.

*The base handling.* `on_client_msg_received` notifies the debug network listener for every message **except** `EMsg::Multi` — a multi is logged after it has been decompressed and unpacked, which is what the C# comment says — and then dispatches on the message type: `Multi` → `handle_multi`, `ClientLogOnResponse` → `handle_logon_response`, `ClientLoggedOff` → `handle_logged_off`, `ClientServerUnavailable` → `handle_server_unavailable`, `ClientSessionToken` → `handle_session_token`. Everything else falls through and the method returns `true`.

*The handlers.*

* `handle_multi` — unpacks a `Multi` message. A non-proto multi is logged ("HandleMulti got non-proto MsgMulti!!") and ignored. The payload is `CMsgMulti.message_body`; if `size_unzipped > 0` the payload is gunzipped with `flate2` (`decompress_gzip`), and a decompression failure is logged and the multi is dropped — the connection survives. The unpacked data is a sequence of length-prefixed sub-messages: a 4-byte little-endian length, then that many bytes. Each sub-message is parsed with `get_packet_msg` and dispatched recursively; a `None` from `get_packet_msg` is handed to `dispatch_client_msg(None)` — the C#'s `OnClientMsgReceived( null )` — which disconnects. (It used to just return `false`, which ended the dispatch loop and left the connection open with nobody reading it.) Like the C#, the `Multi` itself is then handed to the handlers too, after the messages inside it. The recursion is **boxed** — `Box::pin(self.dispatch_client_msg(...))` — because "without it the multi unpacking and the message dispatch form an infinitely sized async recursion": the size of an `async fn`'s future must be known at compile time, and the mutual recursion between the two functions would otherwise have no finite type. The C# recursion is ordinary synchronous recursion and needs no such treatment. The port also adds two bounds checks the C# does not have: a non-positive `sub_size` ends the loop, and a `sub_size` that would run past the end of the buffer ends the loop.
* `handle_logon_response` — a non-proto logon response is logged ("Got non-proto logon response, this is indicative of no logon attempt after connecting.") and dropped: the C# says such a response always fails, so there is nothing to do. A proto response is decoded as `CMsgClientLogonResponse` and its `eresult` read as an `EResult` (defaulting to `EResult::Invalid`). On `EResult::OK` the client stores the session id and the SteamID **from the protobuf header** as `Some(0)` when absent (`unwrap_or(0)`), like the C# value types, plus `cell_id`, `public_ip` (via `NetHelpers::get_ip_address_from_msg`), `ip_country_code` and `user_country_code` from the body, and restarts the heartbeat with `legacy_out_of_game_heartbeat_seconds`, Any other result except the two below means the logon was refused — a wrong password, an expired token, a Steam Guard code that is needed — and the logon is not repeated, because it would fail the same way (and repeating a wrong password is how an account gets rate limited). On `EResult::TryAnotherCM` or `EResult::ServiceUnavailable` it marks the current endpoint as `ServerQuality::Bad` in the server list and the session ends; the caller reconnects, as in C#.
* `handle_logged_off` — clears the session id, the SteamID, the cell id, the public IP and both country codes, stops the heartbeat: a logged-off session is never logged back on automatically (`LoggedInElsewhere` in particular — logging back on would kick the other session, which would then do the same). Then, for a proto message, a `TryAnotherCM`/`ServiceUnavailable` result marks the server bad. The C# clears exactly the same fields.
* `handle_server_unavailable` — logs "A server was not available for request: '<message type>'" and calls `disconnect_with(false)`. The C# also decodes the `MsgClientServerUnavailable` body and logs the server type and the original `EMsg`; the port logs the message type instead, because the body is a *struct* message (`ClientMsg<MsgClientServerUnavailable>`) that this file does not decode.
* `handle_session_token` — decodes `CMsgClientSessionToken` and stores `body.token` in the `AtomicU64`.
* `handle_connected` — `connected_handled.swap(true, Ordering::SeqCst)` makes the handler idempotent, so a duplicate `Connected` event is ignored; the endpoint is marked `ServerQuality::Good` in the server list; `is_connected` is set to true; and `on_client_connected()` runs. 
* `on_client_connected()` — builds `ClientMsgProtobuf<CMsgClientHello>` with `protocol_version = Some(CURRENT_PROTOCOL)` (65581) and sends it, then calls `dispatcher.on_client_connected(self)`. The C# `OnClientConnected` does the same, and the `SteamClient` override raises the `ConnectedCallback`.
* `teardown(connection: Option<&Arc<dyn Connection>>, user_initiated)` — sets `is_connected` to false; if the disconnection was **not** user-initiated *and* `expect_disconnection` is false, marks the endpoint `ServerQuality::Bad`; clears the session id and the SteamID; stops the heartbeat; and calls `on_client_disconnected(user_initiated || expect_disconnection)`. Its callers, `end_session` and `disconnect_with`, make sure it runs once per session. The C# `Disconnected` handler does the same, and the `|| ExpectDisconnection` is what keeps a server-initiated close that the caller *asked for* from being reported as an unexpected drop.

*The heartbeat.* `restart_heart_beat(delay)` stops the previous heartbeat, treats a zero delay as a one-shot (a single heartbeat, then return), and spawns a task that owns a `tokio::time::interval(delay)` with `MissedTickBehavior::Delay` — a late tick does not produce a burst of catch-up ticks. On every tick the task sends `ClientMsgProtobuf<CMsgClientHeartBeat>`; the loop ends when the token is cancelled. The task is started through `tokio::runtime::Handle::try_current()`, and **if there is no runtime the function returns without starting a heartbeat at all** rather than panicking. `stop_heart_beat()` cancels the token and aborts the join handle.

The C# uses `ScheduledFunction`, whose `Start()` calls `timer.Change( TimeSpan.Zero, Delay )` — the due time is zero, so the first tick fires immediately and the period is `Delay`. The Rust `tokio::time::interval` also fires its first tick immediately, so the two agree. They differ in one place: the C# throws `ArgumentOutOfRangeException` out of `Timer.Change` when `Delay` is zero, whereas the port clamps it to a second.

**Notes and gotchas:**

* **The Rust `CMClient` is a concrete struct, not an abstract base class.** `SteamClient` owns an `Arc<CMClient>`, and a `SteamClientDispatcher` bridges the callbacks back through a `Weak<SteamClient>`; the C# `SteamClient` *derives from* `CMClient` and overrides the three `protected virtual` members. The consequence is that the C#'s `base.OnClientMsgReceived( packetMsg )` — which the C# `SteamClient` calls first, before its own handling — becomes an explicit two-step in the Rust `dispatch_client_msg`: base handling, then dispatcher.
* **The C# `Connect` returns `void`; the Rust `connect` returns `Result<(), SteamError>`.** The C# reports the outcome through the `ConnectedCallback`/`DisconnectedCallback` and swallows the exception into the continuation's `IsFaulted` branch; the Rust port awaits the socket and hands the failure back to the caller *and* raises the disconnected callback.
* **The C# serialises concurrent connects with `lock ( connectionLock )`.** The Rust `connect` has no such lock; it relies on the `CancellationToken` and the `RwLock`s. Calling `connect` twice concurrently from two tasks is therefore a race the C# explicitly asserts against ("Connection was set during a connect, did you call CMClient.Connect() on multiple threads?").
* **The C# constructor rejects a null configuration and an empty identifier** (`ArgumentException("Identifer must not be empty.")`). The Rust `CMClient::new` accepts an empty identifier — the check moved up to the `SteamClient` constructors, which replace an empty identifier with a generated one — so the C#'s exception has no Rust counterpart at this level.
* **The C# subscribes and unsubscribes the three connection events** (`+=` in `Connect`, `-=` in `Disconnected`). The Rust port subscribes one ordered event stream per connection and drops it when the dispatch loop ends; there is nothing to unsubscribe.
* **The C# swallows send failures; the port reports them.** `CMClient::send` returns `Err(SteamError::Connection(..))` when there is no outbound queue, or when the queue has been closed.
* **The C#'s `Disconnect` waits for the pending connection-setup task.** The Rust `disconnect_with` cancels the token and tears down, but it does not wait for a `connect` that is still awaiting the server list.
* **The multi-message unpacker is bounds-checked and its recursion is boxed.** The C# trusts the length prefixes it reads out of the payload.
* **Nothing in this file panics on runtime-supplied data.** `get_packet_msg` returns `Option`; `handle_multi` breaks out of its loop on a bad length; `stamp_identity` returns the bytes unchanged on an unrecognised layout; `EMsg::from_value` falls back to `EMsg::Invalid`; `decompress_gzip` maps the failure to `SteamError::Decompression`; `restart_heart_beat` treats a zero delay as a one-shot and copes with the absence of a runtime. This is the crate-wide rule stated in `error.rs`.
* **`handle_server_unavailable` logs less than the C#.** The C# decodes the `MsgClientServerUnavailable` body and prints the server type and the `EMsg` that was sent; the port prints the message type only.
* **`CURRENT_PROTOCOL` is part of the wire protocol.** It is `pub`, so a sample can read it, but changing it would break every Steam server.
* **The heartbeat delay comes from the server.** `legacy_out_of_game_heartbeat_seconds` in the `ClientLogonResponse` is the only source of the interval; the port does not clamp it to a maximum, so a hostile or broken server could ask for a very short interval. The C# has the same exposure.
- **A callback-type mismatch requests a disconnect.** `post_callback` calls `cm.request_disconnect()` when `try_complete_job` reports one; `on_client_msg_received` consumes the flag after the handler loop and calls `disconnect_with(false)`, matching the C# `InvalidCastException` path.
* **Every CM message is logged, in both directions.** `send` writes `CM: --> <MsgType>` and the dispatch loop writes `CM: <-- <MsgType>`, matching the C# `DebugLog` lines. The port did not do this before; it does now.
* **The transport order is a deviation from the C#.** The C# tries WebSocket, then TCP, then UDP; the port tries **TCP, then the websocket, then UDP**, because the server closes a websocket CM session after a minute or two while TCP holds it. Both transports complete a logon. the porting notes §13 records the measurement.

### 5.5 `crates/steamkit/src/networking/steam3/` — the Steam3 transports


### 5.6 `crates/steamkit/src/steam/steam_client/` — the client, the jobs, the callbacks


#### `crates/steamkit/src/networking/steam3/` — the Steam3 transport layer

This directory is the Rust port of the C# `SteamKit2/Networking/Steam3/` namespace. It holds everything below the CM client that actually moves bytes: the connection abstraction (`i_connection.rs`), three concrete transports (TCP, UDP, WebSocket), the Steam channel-encryption layer that wraps any transport (`envelope_encrypted_connection.rs`, together with the `NetFilterEncryptionWithHMAC` cipher it drives), and the small value types that travel over the wire (`protocol_type.rs`, `net_msg_event_args.rs`, `disconnected_event_args.rs`).

The layering, outermost first:

1. `CMClient` (`steam/cm_client.rs`) drives the connection. It builds one through the `ConnectionFactory` seam and subscribes to the connection's ordered event stream before the socket is opened. The connection and its stream travel together in a private `ConnectionSession`, which the inbound dispatch loop **owns** — that is what lets the loop replace the whole session when Steam answers a logon with "connect to a different CM server".
2. `EnvelopeEncryptedConnection` wraps a raw transport and performs Steam's channel-encryption handshake. Once the handshake completes, every frame is AES-256-CBC with an HMAC-derived IV.
3. The raw transports — `TcpConnection`, `UdpConnection`, `WebSocketConnection` — implement `IConnection` and speak their own framing.
4. `ConnectionEventSenders` carries the three C# events (`NetMsgReceived`, `Connected`, `Disconnected`): as one ordered, lossless stream of `ConnectionEvent`s per subscriber — which is what the library itself consumes — and, for callers that want a single kind of event, as three `tokio::sync::broadcast` channels.

Every type here is `unsafe`-free (the crate is `#![forbid(unsafe_code)]`), and none of it panics on network input: malformed frames are logged and dropped, and every fallible path returns `SteamError`.

#### `crates/steamkit/src/networking/steam3/mod.rs` — 12 lines
**C# counterpart:** none — the C# has no module manifest; the Rust file is the `pub mod` list that makes the directory's eleven submodules reachable as `steamkit::networking::steam3::<name>`.
**Purpose:** Declares the eleven submodules of the Steam3 transport layer.
**How it works:** One `pub mod` line per source file: `disconnected_event_args`, `envelope_encrypted_connection`, `i_connection`, `net_filter_encryption_with_hmac`, `net_msg_event_args`, `protocol_type`, `tcp_connection`, `udp_connection`, `udp_packet`, `web_socket_connection`, `web_socket_context`. There is no re-export block, so every type is addressed by its full path.
**Notes and gotchas:** The C# `SteamKit2` namespace is flat — `IConnection`, `TcpConnection`, `UdpPacket` and friends all live in the same namespace, so a C# file can use them without qualification. The Rust port keeps the C# file layout 1:1, which means the same types are spread across eleven modules and must be imported by path. Note that the C# `SteamKit2.Internal` namespace (which owns `MsgHdr`, `MsgChannelEncryptRequest`, `UdpHeader`, `ChallengeData`, `ConnectData`) has no counterpart here except the three UDP structs that `udp_packet.rs` re-implements locally.

#### `crates/steamkit/src/networking/steam3/disconnected_event_args.rs` — 15 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/DisconnectedEventArgs.cs`
**Purpose:** The payload of the `Disconnected` event, carrying one bit: whether the disconnection was requested by the consumer. It is the only argument the three connection events share besides the net message payload.
**How it works:** A `struct DisconnectedEventArgs { pub user_initiated: bool }` with a `new(user_initiated: bool)` constructor. It derives `Clone, Copy, Debug, PartialEq, Eq`; the shape is fixed by `SUMMARY.md` §8.
**Notes and gotchas:** The C# class derives from `System.EventArgs` and exposes a get-only `UserInitiated` property. The Rust type is a plain struct with no `EventArgs` base (there is no such thing in Rust) and is `Copy` — `Copy` is required in practice because the value is pushed through a `broadcast::Sender`, whose payload bound is `Clone`, and `Copy` is the cheapest way to satisfy that. `Copy` also means a subscriber can keep the value after the channel drops it.

#### `crates/steamkit/src/networking/steam3/net_msg_event_args.rs` — 27 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/NetMsgEventArgs.cs`
**Purpose:** The payload of the `NetMsgReceived` event: the raw bytes that arrived from the network and the endpoint they arrived from.
**How it works:** A `struct NetMsgEventArgs { pub data: Vec<u8>, pub end_point: SocketAddr }` with `new(data, end_point)` and `with_data(&self, data) -> NetMsgEventArgs`, which returns a copy carrying different payload bytes but the same endpoint. Derives `Clone, Debug, PartialEq, Eq`.
**Notes and gotchas:** `with_data` is the direct counterpart of the C# `NetMsgEventArgs WithData(byte[])` helper, which `EnvelopeEncryptedConnection` uses to swap the ciphertext for the decrypted plaintext before re-raising the event. The C#'s `EndPoint` is an abstract type that may be an `IPEndPoint` or a `DnsEndPoint`; the Rust `SocketAddr` is always a resolved address, so the hostname that a `DnsEndPoint` carried is gone by the time the event is raised (see `i_connection.rs`). The payload is a `Vec<u8>`, not `Bytes`: the C# allocates a fresh `byte[]` per packet too, but the Rust type is cloned into every `broadcast` subscriber, so a `Bytes` would have been cheaper. `Bytes` is used on the *send* path (`IConnection::send`), not here.

#### `crates/steamkit/src/networking/steam3/protocol_type.rs` — 67 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/ProtocolType.cs`
**Purpose:** Models the C# `[Flags] enum ProtocolTypes`, which selects which transports a client is allowed to use when connecting to a CM server.
**How it works:** `pub struct ProtocolTypes(pub u32)` — a newtype over the raw bit mask, as mandated by `SUMMARY.md` §8. Constants: `NONE = 0`, `TCP = 1 << 0`, `UDP = 1 << 1`, `WEB_SOCKET = 1 << 2`, `ALL = 7`. `raw()` returns the mask; `flags()` yields the individual set flags in `Tcp`, `Udp`, `WebSocket` order (the counterpart of the C# `ProtocolTypesExtensions.GetFlags` iterator). `BitOr` and `BitOrAssign` are implemented so masks combine with `|`. `Default` is `NONE`.
**Notes and gotchas:** `has(other)` is `(self.0 & other.0) > 0` — **any of**, not all of. This is exactly the C# `HasFlagsFast` extension (`(self & flags) > 0`) and is deliberately *not* the stricter BCL `Enum.HasFlag` semantics, which requires every bit of the argument to be present. The distinction matters: `ProtocolTypes::ALL.has(ProtocolTypes::TCP | ProtocolTypes::UDP)` is `true` here, because some bits overlap, even though `ALL` also contains `WebSocket`. Callers that want "contains exactly this set" must compare `raw()` values. `ProtocolTypes::NONE.has(x)` is `false` for every `x` except `NONE` itself (`0 & 0 > 0` is still false), so `NONE` never matches anything. `flags()` uses `has`, so a mask with an unknown bit set silently ignores that bit. The struct is `Copy + Eq + Hash + Ord`, so it can be a map key.

#### `crates/steamkit/src/networking/steam3/i_connection.rs` — 200 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/IConnection.cs`
**Purpose:** Defines the transport-agnostic connection interface that the CM client programs against, plus the plumbing that carries the three C# connection events. The C# interface mixes the operations and the events in one type; the Rust port splits them, because `SUMMARY.md` §8 freezes the shape of `IConnection` and that shape has no event methods.
**How it works:** Three traits and one helper struct.

- `IConnection` (frozen by contract §12, `#[async_trait]`): `current_end_point(&self) -> Option<SocketAddr>`, `protocol_types(&self) -> ProtocolTypes`, `async connect(&self, end_point: SocketAddr, timeout: Duration) -> Result<(), SteamError>`, `async disconnect(&self, user_initiated: bool)`, `async send(&self, data: Bytes) -> Result<(), SteamError>`, `local_ip(&self) -> Option<IpAddr>`. The trait requires `Send + Sync`.
- `ConnectionEvent`: `NetMsgReceived(NetMsgEventArgs)`, `Connected`, `Disconnected(DisconnectedEventArgs)` — one event, as it travels on the ordered stream.
- `ConnectionEvents`: the subscription methods. `events(&self) -> mpsc::UnboundedReceiver<ConnectionEvent>` is the ordered, lossless stream of all three events, and the one to drive a protocol with. `net_msg_received(&self) -> broadcast::Receiver<NetMsgEventArgs>`, `connected(&self) -> broadcast::Receiver<()>` and `disconnected(&self) -> broadcast::Receiver<DisconnectedEventArgs>` are one broadcast receiver per event. Every `IConnection` implementation in the crate also implements `ConnectionEvents`.
- `Connection`: the blanket convenience trait `pub trait Connection: IConnection + ConnectionEvents {}`, with `impl<T> Connection for T where T: IConnection + ConnectionEvents + ?Sized`. `EnvelopeEncryptedConnection` takes its inner transport as `Arc<dyn Connection>` because it needs both halves.
- `ConnectionEventSenders`: the storage behind every concrete transport's `ConnectionEvents` implementation. It holds three `broadcast::Sender`s (`NetMsgEventArgs`, `()`, `DisconnectedEventArgs`) and the list of ordered-stream subscribers (`Arc<parking_lot::Mutex<Vec<mpsc::UnboundedSender<ConnectionEvent>>>>`), and offers `new(capacity)`, `with_default_capacity()` (`CONNECTION_EVENT_CAPACITY = 64`, the capacity of each broadcast channel), `subscribe_events()`, `subscribe_*()`, `event_subscriber_count()`, and `emit_net_msg_received` / `emit_connected` / `emit_disconnected`. An emitter holds the subscriber lock while it hands the event to every subscriber, which gives events raised from different tasks one order that every subscriber agrees on; closed subscribers are dropped from the list as a side effect. An event raised with no subscribers goes nowhere — exactly like a C# `event` with no subscribers. `ConnectionEventSenders` is `Clone + Debug` (clones share the channels), so it can be moved into a spawned task.

**Notes and gotchas:**

- **`connect` takes a `SocketAddr`, which loses the hostname at the transport layer.** The C# signature is `void Connect(EndPoint endPoint, int timeout = 5000)`, and `EndPoint` is either an `IPEndPoint` or a `DnsEndPoint`. A `DnsEndPoint` keeps the host name, which is (a) the TLS server name for the WebSocket transport, and (b) the value the C# resolves at connect time (`UdpConnection.Connect` calls `Dns.GetHostAddresses`; `CMClient` resolves the CM host before connecting). Contract §12 froze the trait signature to `SocketAddr`, so the name is gone there. Two consequences: the WebSocket transport is handed the host name separately (`WebSocketConnection::new_for_host` / `new_tungstenite_for_host`, `WebSocketContext::new_with_uri`), while `CMClient::current_end_point()` reconstructs `Option<EndPoint>` from the server record; and DNS resolution moved earlier - the Rust `CMClient::connect` resolves the server record host when the record is created, not when the socket is opened.
- **Timeout units changed.** The C# takes `int timeout = 5000` in *milliseconds*; the Rust takes `Duration`, so a caller that used to pass `5000` must now pass `Duration::from_millis(5000)`. `TcpConnection` maps the elapse to `SteamError::Timeout(timeout)` and a refused connection to `SteamError::Connection`.
- **`send` takes `Bytes`, not `Memory<byte>`.** `Bytes` is a cheap-to-clone, refcounted buffer, so the write path does not have to copy the frame.
- **Event delivery is asynchronous.** A C# `event` is invoked synchronously, on the raising thread, **in subscription order**; the handler has finished running by the time the raising call returns, so a handler sees every event, in order, and the raiser waits for it. Here the raising call hands the event to each subscriber's queue and returns immediately. The ordered stream keeps the rest of the C# guarantee — every event, in the order raised — at the price of an unbounded queue: a subscriber that falls behind buffers instead of losing events.
- **The broadcast receivers are lossy, and must not drive a protocol.** A broadcast subscriber that falls more than `CONNECTION_EVENT_CAPACITY` events behind gets `RecvError::Lagged(n)` and loses them (contract §5.2 rule 3 requires skipping, never panicking), and the three channels are not ordered relative to each other. The library itself used to consume them: the CM dispatch loop lost every message past the 64th of a burst that arrived while a handler was busy, and the encryption layer handled `Connected` and the first message on two different tasks, so the server's `ChannelEncryptRequest` could be rejected for arriving "before" `Connected`. Everything inside the crate now uses `events()`; the porting notes §14 records the change.
- **One stream carrying an enum, plus three channels.** The ordered stream exists because only a single queue can carry the order between different kinds of event. The three broadcast channels are kept for callers that want only one kind; a subscriber of `NetMsgReceived` is never woken by a `Connected` event. Every subscription must be made before `connect`, or the first events are missed.
- **`Connected` carries `()`, not `EventArgs`.** There is nothing in the C#'s `Connected` event payload either, and `EventArgs` has no Rust counterpart.
- **Concrete transports also expose inherent `subscribe_*` methods** (for example `TcpConnection::subscribe_net_msg_received`) in addition to the trait methods, so a caller holding a concrete type does not have to import the `ConnectionEvents` trait to subscribe.
- **`ConnectionEventSenders` is not `Default`-less**: it implements `Default` as `with_default_capacity()`, so `Default::default()` and `with_default_capacity()` are the same thing.

#### `crates/steamkit/src/networking/steam3/net_filter_encryption_with_hmac.rs` — 271 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/NetFilterEncryptionWithHMAC.cs`
**Purpose:** Implements Steam's channel cipher: AES-256-CBC with PKCS#7 padding for the payload, and an HMAC-SHA1 that is folded into the initialization vector rather than appended as a separate MAC field.
**How it works:** `struct NetFilterEncryptionWithHMAC { session_key: [u8; 32], hmac_secret: [u8; 16] }`. `new(session_key)` rejects a key that is not exactly 32 bytes with `SteamError::InvalidArgument`, copies it, and derives `hmac_secret` from its **first 16 bytes**.

Wire format:

```text
offset  size  field
0       16    AES-256-ECB(session_key, iv)
16      ..    AES-256-CBC-PKCS7(session_key, iv, plaintext)
```

where the 16-byte plaintext `iv` is:

```text
offset  size  field
0       13    HMAC-SHA1(hmac_secret, iv[13..16] || plaintext)[0..13]
13      3     random bytes
```

- `process_outgoing(data)` allocates `calculate_max_encrypted_data_length(data.len())` bytes, calls `process_outgoing_into`, and truncates to the bytes actually written. `process_outgoing_into(data, output)` is the buffer-reusing form and returns the byte count.
- `generate_initialization_vector` fills `iv[13..16]` with 3 random bytes from `rand::thread_rng()`, computes `compute_hmac(iv, plaintext)` over `iv[13..16] || plaintext`, and writes the first 13 bytes of the 20-byte tag into `iv[0..13]`. The IV is then ECB-encrypted into `output[0..16]` and the plaintext is CBC-encrypted with PKCS#7 into `output[16..]`.
- `process_incoming(data)` requires at least 16 bytes, ECB-decrypts `data[0..16]` to recover the IV, CBC-decrypts `data[16..]` with that IV, then calls `validate_initialization_vector`, which recomputes the HMAC over `iv[13..16] || plaintext` and compares its first 13 bytes against `iv[0..13]` **in constant time** (`Mac::verify_truncated_left`), so the time the check takes does not reveal how much of a forged tag was right. A mismatch yields `SteamError::Crypto("HMAC from server did not match computed HMAC.")`. The C# compares with `SequenceEqual`, which stops at the first difference.
- `calculate_max_encrypted_data_length(n) = 16 + (n + 16) / 16 * 16`, i.e. the IV plus room for the ciphertext **and** one full block of PKCS#7 padding.
- `hmac` feeds `iv[13..16]` and then the plaintext into HMAC-SHA1 (`hmac::Hmac<Sha1>`) keyed with `hmac_secret`, without copying the plaintext into a separate buffer; `compute_hmac` finalises it into the 20-byte tag.
- The AES helpers `ecb_encrypt` / `ecb_decrypt` / `cbc_encrypt` / `cbc_decrypt` construct a fresh `aes::Aes256` per call. `cbc_decrypt` rejects an empty ciphertext or one whose length is not a multiple of 16, and maps a bad pad to `SteamError::Crypto`.

**Notes and gotchas:**

- **There is no MAC over the ciphertext.** The HMAC covers only the 3 random IV bytes plus the *plaintext*, and it is truncated to 13 bytes and stored *inside* the IV. This is Steam's "HMAC IV" scheme; a reader expecting a 20-byte tag appended to the frame will misparse it. Integrity of the ciphertext rests on CBC mode plus the IV structure — a bit-flip in the ciphertext is only detected indirectly (through the padding check).
- **The HMAC secret is derived from the session key**, not independent: `hmac_secret = session_key[0..16]`. The C# does the same (`Array.Copy(sessionKey, 0, hmacSecret, 0, hmacSecret.Length)`). Anyone reimplementing this against a different session key layout will produce frames Steam rejects.
- **The C# constructor only asserts the key length.** `DebugLog.Assert(sessionKey.Length == 32, ...)` merely logs. The Rust port returns `SteamError::InvalidArgument`, which is a deliberate hardening: a wrong key length can no longer produce a frame that is silently wrong.
- **Error propagation differs.** The C# `ProcessIncoming` catches `CryptographicException` and rethrows it as an `IOException` so that the network thread handles it. The Rust port returns `SteamError::Crypto`, and `EnvelopeEncryptedConnection` logs the error and drops the packet without tearing the connection down.
- **The C# shares one `Aes` instance under a `lock`**; the Rust port builds a new cipher per operation. `aes::Aes256` holds only the expanded key schedule, so there is no shared mutable state and no lock is needed.
- **Allocation profile.** The C# rents its output buffer from `ArrayPool<byte>.Shared`; the Rust port allocates a fresh `Vec` per outgoing packet and returns it. This is a throughput difference, not a wire difference.
- `NetFilterEncryptionWithHMAC` implements `Clone` and a manual `Debug` that does not print the key material (`finish_non_exhaustive`).

#### `crates/steamkit/src/networking/steam3/envelope_encrypted_connection.rs` — 489 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/EnvelopeEncryptedConnection.cs`
**Purpose:** A decorator that sits on top of any `IConnection` and performs Steam's channel-encryption handshake. It answers the server's `ChannelEncryptRequest` with an RSA-OAEP-SHA1 blob carrying a fresh 32-byte session key, and from then on every frame is AES-256-CBC + HMAC-SHA1.
**How it works:**

Shared state lives in `struct EnvelopeShared { inner: Arc<dyn Connection>, events: ConnectionEventSenders, universe: EUniverse, state: Mutex<EncryptionState>, encryption: Mutex<Option<Arc<NetFilterEncryptionWithHMAC>>>, debug_network_listener: Option<Arc<dyn DebugNetworkListener>>, forwarders_started: AtomicBool }`, wrapped in `Arc`; `EnvelopeEncryptedConnection { shared: Arc<EnvelopeShared> }`.

- `EncryptionState` is the four-state machine of the C#: `Disconnected`, `Connected`, `Challenged`, `Encrypted`.
- `new(inner, universe)` and `with_debug_network_listener(inner, universe, listener)` build the shared state. The `DebugNetworkListener` type is re-exported from the util module: `pub use crate::util::debug_network_listener::IDebugNetworkListener as DebugNetworkListener;` (the C# counterpart is `SteamKit2.Internal.IDebugNetworkListener`).
- `spawn_forwarders` subscribes to the inner connection's ordered event stream and starts **one** tokio task that handles its events in order, on the **first `connect`** rather than in the constructor — so constructing a connection does not require a tokio runtime. `forwarders_started` is an `AtomicBool` compared with `compare_exchange`, making the call idempotent. `Connected` sets `state = Connected`; `Disconnected` sets `state = Disconnected`, clears `encryption`, and re-emits `Disconnected`; `NetMsgReceived` calls `handle_net_msg_received`. The task holds the shared state **weakly** and ends once the connection is dropped.
- `IConnection::send` passes the data through untouched while `state != Encrypted`. Once encrypted, it takes the cipher out of the mutex, calls `process_outgoing`, and forwards the ciphertext to the inner connection. If the state says "encrypted" but the cipher is gone (a disconnect raced the send), it logs `Attempting to send client data when not connected.` and returns `Ok(())`.
- `handle_net_msg_received` has two modes. In `Encrypted` state it decrypts with `process_incoming` and re-emits the event with `args.with_data(plaintext)`. Otherwise it parses the `EMsg` from the first four bytes — `parse_msg_type` masks off the protobuf bit `PROTO_MASK = 0x8000_0000` — notifies the debug listener, and rejects the message unless `is_expected_emsg(state, msg_type)` holds: `Disconnected → false`, `Connected → only ChannelEncryptRequest`, `Challenged → only ChannelEncryptResult`, `Encrypted → true`. A `process_incoming` failure is logged and disconnects the inner connection.
- `handle_encrypt_request` reads the protocol version (`u32` at offset 20) and the universe (`i32` at offset 24) from the plain `MsgHdr`; the challenge is everything from offset 28. It generates a random 32-byte session key, builds `blob = session_key || challenge`, encrypts it with RSA-OAEP-SHA1 using the universe's public key, computes `crc32fast::hash` of the blob, installs a `NetFilterEncryptionWithHMAC` built from the same session key, sets `state = Challenged`, and sends the response.
- `build_channel_encrypt_response` serializes the 20-byte `MsgHdr` by hand and appends the body: `EMsg::ChannelEncryptResponse` (i32), target job id `u64::MAX`, source job id `u64::MAX`, protocol version `1` (u32), key size `128` (u32), the 128-byte RSA blob, the CRC32 (u32), and a zero (u32).
- `handle_encrypt_result` reads the `EResult` at offset 20. `EResult::OK` **and** a cipher present → `state = Encrypted` and `emit_connected()`; otherwise log `Encryption channel setup failed` and `inner.disconnect(false)`.
- `public_key_for_universe(universe)` returns the DER-encoded RSA public key for `Public`, `Beta`, `Internal` and `Dev` from `KeyDictionary`; `Invalid` returns `None`.

**Notes and gotchas:**

- **The `Connected` event is raised only after the handshake completes.** `EnvelopeEncryptedConnection` does not forward the inner connection's `Connected`; it raises its own when the channel becomes encrypted. The C# does the same (`Connected?.Invoke` inside `HandleEncryptResult`). A `CMClient` therefore reports "connected" only when the channel is actually encrypted — this is the semantic that makes `expect_disconnection` and the logon handshake work.
- **Two of the C#'s three `DebugLog.Assert` checks became debug logs.** The C# asserts `protoVersion == 1` and `connectedUniverse == universe`; `DebugLog.Assert` only logs and does not abort, and the Rust port reproduces that exactly (log and continue). The third assert — that the challenge payload is at least 16 bytes — *is* enforced in the Rust port (`MIN_CHALLENGE_LENGTH = 16`): a short challenge disconnects the connection. That is a deliberate hardening, and it is the one place where the C# would have proceeded with a malformed challenge.
- **The Rust port parses only the `EMsg`; the C# parses the whole packet.** The C# calls `CMClient.GetPacketMsg(e.Data, log)` and rejects the message when that returns `null`. The Rust port reads four bytes and rejects only when those four bytes are not a valid `EMsg`. A message that the C# would have rejected as unparseable is accepted here (and then dropped by `is_expected_emsg` anyway, because it would not be `ChannelEncryptRequest`/`ChannelEncryptResult`).
- **The forwarder starts on the first `connect`, not at construction.** The C# subscribes to the inner connection's events in its constructor, so a `Connected` event raised between construction and `Connect` would be seen. In the Rust port that window is closed. In practice the inner connection raises nothing before `connect`.
- **One forwarder, in order — the handshake depends on it.** The server sends its `ChannelEncryptRequest` the moment the socket is open, right behind the transport's `Connected`, and the request is only accepted once `Connected` has moved the state out of `Disconnected`. The port used to run three forwarders, one per event, on three broadcast channels, so the request could be handled first and silently rejected, stalling the handshake until the connection timed out. `tests/envelope_encrypted_connection.rs` raises the two back to back, fifty times.
- **The forwarder used to leak.** It held the shared state strongly, the shared state holds the inner connection, and the inner connection holds the sender of the forwarder's stream — a cycle, so every connection ever made kept a task and its buffers alive. It now holds a `Weak`.
- **The `Send` race is gone.** The C# comment says a disconnect on the network thread can clear `encryption` between the state check and the use, and works around it with a local copy. The Rust port takes the `std::sync::Mutex` and clones the `Arc` out of it, so the cipher cannot be freed mid-send.
- **No mutex guard is held across an `.await`.** The `state` and `encryption` mutexes are `std::sync::Mutex`, and every access is a statement-scoped temporary; the `.await` on `inner.send(...)` happens after the guards have been dropped. This satisfies contract §5.2 rule 4.
- **`public_key_for_universe` reads `KeyDictionary`.** It used to carry its own copy of the four universe keys, and that copy's `Dev` key was corrupt — 146 bytes instead of 160, differing from the C# in 123 of them — so a handshake with a `Dev` universe server could never succeed. The duplicate is gone; `tests/msg_util.rs` checks that the two functions agree and `tests/envelope_encrypted_connection.rs` answers a request in every universe.
- **`KEY_SIZE = 128`** is the size of the RSA blob in the response. The C# derives the same number implicitly from the RSA modulus; here it is a constant written into the response body.
- **The debug listener is notified for the handshake messages too** — `on_incoming_network_message(msg_type, &args.data)` for the incoming request and `on_outgoing_network_message(EMsg::ChannelEncryptResponse, &serialized)` for the response. In the C# the listener is called with the fully parsed packet; here the raw bytes are passed.
- **The C# catches exceptions thrown by the debug listener** and logs them. The Rust port calls the listener without a catch, so a panicking listener implementation would unwind the forwarder task. The crate's own `NetHookNetworkListener` does not panic.

#### `crates/steamkit/src/networking/steam3/proxy.rs` — 479 lines
**C# counterpart:** none - it is implicit in `System.Net.Http.HttpClient`.
**Purpose:** Resolves the conventional proxy environment variables and opens an HTTP `CONNECT` tunnel so the WebSocket transport can reach the CM through a proxy, the way C# does through `HttpClient`/`ClientWebSocket`.
**How it works:** `proxy_for(host, port)` reads `https_proxy`, `HTTPS_PROXY`, `all_proxy`, `ALL_PROXY` in that order, honours `no_proxy`/`NO_PROXY`, and parses only `http://` URLs (optional `user:pass@`, IPv6 brackets, default port 8080). `connect_through` dials the proxy, writes a `CONNECT host:port HTTP/1.1` request, reads the response headers one byte at a time so no tunnel bytes are consumed, and returns the `TcpStream` once the status line is `HTTP/1.x 200`. Both the dial and the handshake are bounded by a 10 second timeout.
**Notes and gotchas:**

- **Environment variables only.** The Windows registry (`WinINET`) and macOS `SystemConfiguration` cases that `HttpClient.DefaultProxy` also consults are deliberately out of scope; a proxy configured only in the OS settings is not picked up.
- **`http://` proxies only.** `https://` and `socks*://` URLs are rejected rather than guessed at.
- **The byte-at-a-time read is deliberate.** Reading ahead would consume TLS bytes that belong to the tunnel.

#### `crates/steamkit/src/networking/steam3/tcp_connection.rs` — 323 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/TcpConnection.cs`
**Purpose:** The raw TCP transport. It frames messages with Steam's "VT01" header and raises the three connection events.
**How it works:**

Steam's TCP framing is an 8-byte header followed by the payload:

```text
offset  size  field
0       4     payload length (u32, little endian)
4       4     magic 0x31305456 ("VT01", little endian)
8       ..    payload
```

- `struct TcpConnection { events: ConnectionEventSenders, inner: Arc<TcpInner> }` with `struct TcpInner { writer: tokio::sync::Mutex<Option<OwnedWriteHalf>>, current_end_point: Mutex<Option<SocketAddr>>, local_ip: Mutex<Option<IpAddr>>, cancel: Mutex<Option<CancellationToken>>, io_timeout: Mutex<Duration>, disconnect_notified: AtomicBool }`.
- `connect(end_point, timeout)` wraps `TcpStream::connect(end_point)` in `tokio::time::timeout`. A refused connection becomes `SteamError::Connection("failed connecting to {end_point}: {err}")`; an elapse becomes `SteamError::Timeout(timeout)`. It splits the stream, records the endpoint and the local IP, installs a fresh `CancellationToken`, stores the write half, resets `disconnect_notified`, raises `Connected`, and only then spawns `tcp_net_loop` — so that a message the server sends straight away, or a `Disconnected`, can never be raised ahead of `Connected`.
- `tcp_net_loop` reads the 8-byte header (length + magic) in one `read_exact_timed` call, then the payload in another. The first read waits indefinitely (the C# `Socket.Poll` loop); every following chunk is bounded by `io_timeout` (the C# `ReceiveTimeout`), and cancellation wins over a pending read. A magic mismatch logs `Got a packet with invalid magic!` and ends the loop; a length above `MAX_PACKET_SIZE` (16 MiB) logs `Got a packet with an implausible length!` and ends the loop.
- `send(data)` builds the 8-byte header plus the payload into one buffer and does `write_all` + `flush` on the write half, all inside `tokio::time::timeout(io_timeout, ...)`; an elapse is `SteamError::Timeout`. When there is no write half it logs `TcpConnection: Attempting to send client data when not connected.` and returns `Ok(())`.
- `disconnect(user_initiated)` cancels the token, drops the write half, and raises `Disconnected` exactly once — guarded by `disconnect_notified.swap(true, Ordering::SeqCst)`.
- `protocol_types()` returns `ProtocolTypes::TCP`. `local_ip()` returns the address the socket was bound to.

**Notes and gotchas:**

- **The 16 MiB length cap is Rust-only.** The C# has no such limit: it does `netReader.ReadBytes((int)packetLen)` and would allocate whatever length the peer announced. The Rust port drops the connection instead. This is a hardening against a hostile or broken peer; it also means a legitimate frame larger than 16 MiB would be rejected, which Steam never sends.
- **The C# throws on a bad frame; the Rust port logs and closes.** A wrong magic or a truncated payload raises an `IOException` in C# and tears the net thread down. The Rust loop breaks out and the `Disconnected` event is raised with `user_initiated = false`. The observable outcome is the same.
- **`Connected` is raised by `connect` itself**, right after the socket is up and before the read loop is started — the C# raises it from inside the net thread. The order matters: the port used to start the read loop first, and on a multi-threaded runtime the loop could raise the server's first message before `Connected`.
- **Exactly one `Disconnected` event.** Both the explicit `disconnect()` path and the read loop's exit path race for the same flag, so a caller that calls `disconnect(true)` and then sees the socket die still receives a single event — with `user_initiated = true` if the explicit call won the race, `false` otherwise.
- **A send on a dead connection is not an error.** Both the C# and the Rust port silently drop the data. The caller cannot distinguish "sent" from "discarded".
- **The writer mutex is a `tokio::sync::Mutex`** because its guard is held across `write_all().await`; the other three mutexes are `std::sync::Mutex` with statement-scoped guards. This is exactly the split contract §5.2 rule 4 requires.
- **Read and write timeouts follow the C#.** `io_timeout` is the `ConnectionTimeout` passed to `connect` (5 s by default): an idle connection is never closed because the first read is untimed, but a partially received frame or a blocked write is dropped after the timeout. **No DNS resolution happens in this transport.** `TcpStream::connect` takes a `SocketAddr`, so the host name must already have been resolved by the caller (`CMClient` resolves the server record host when the record is created).

#### `crates/steamkit/src/networking/steam3/udp_packet.rs` — 413 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/UdpPacket.cs` (plus the C# `SteamKit2.Internal.UdpHeader`, `ChallengeData` and `ConnectData`, which are generated from `Resources/SteamLanguage/netheader.steamd` and are re-implemented here because the `base` wave does not produce them)
**Purpose:** The Steam UDP datagram format: a fixed 36-byte header plus the payload, and the two small handshake payloads (`ChallengeData`, `ConnectData`) that ride inside it.
**How it works:**

Every UDP datagram is a 36-byte little-endian header followed by the payload:

```text
offset  size  field
0       4     magic          (u32, always 0x31305356)
4       2     payload_size   (u16)
6       1     packet_type    (u8, EUdpPacketType)
7       1     flags          (u8)
8       4     source_conn_id (u32)
12      4     dest_conn_id   (u32)
16      4     seq_this       (u32)
20      4     seq_ack        (u32)
24      4     packets_in_msg (u32)
28      4     msg_start_seq  (u32)
32      4     msg_size       (u32)
36      ..    payload
```

- `UdpHeader` is a `Copy` struct with those eleven public fields, `MAGIC = 0x31305356`, `SERIALIZED_SIZE = 36`, a `new()` that fills in the C# default field values (notably `source_conn_id: 512`), `serialize()` and `deserialize(&mut self, data)`.
- `deserialize` reads the fields in order with a cursor helper (`read_u32` / `read_u16` / `read_u8` / `read_bytes`); the first short read returns `SteamError::Malformed("UDP packet is too short to contain a header")`, and the fields read so far stay populated — mirroring the C# `BinaryReader` behaviour, where a partially read header keeps its values.
- `packet_type_from_u8` / `packet_type_to_u8` map the wire byte to and from `EUdpPacketType`: `Invalid 0, ChallengeReq 1, Challenge 2, Connect 3, Accept 4, Disconnect 5, Data 6, Datagram 7, Max 8`; any other value becomes `Invalid`.
- `ChallengeData { challenge_value: u32, server_load: u32 }` — 8 bytes on the wire, `CHALLENGE_MASK = 0xA426DF2B`, `server_load` is parsed but unused by SteamKit.
- `ConnectData { challenge_value: u32 }` — 4 bytes on the wire, the same `CHALLENGE_MASK`.
- `UdpPacket { header: UdpHeader, payload: Option<Vec<u8>> }` with `new(packet_type)`, `with_payload(packet_type, payload)`, `with_payload_slice(packet_type, source, offset, length)`, `from_bytes(data)`, `is_valid()`, `payload()`, `set_payload(payload)`, `set_payload_slice(source, offset, length)` and `get_data()`.
- `set_payload` and `set_payload_slice` reject a payload above `MAX_PAYLOAD` with `SteamError::InvalidArgument("Payload length exceeds 0x4DC maximum")` and update `header.payload_size` and `header.msg_size` to the payload length.
- `MAX_PAYLOAD = 0x4DC` (1244 bytes) — the largest payload one UDP packet may carry.

**Notes and gotchas:**

- **The header magic is the "obfuscated" marker.** `0x31305356` is `"VS01"` on the wire (`56 53 30 31`). This fixed header — magic, sizes, connection ids, sequence numbers — *is* Steam's UDP framing; there is no separate obfuscation pass in this port and no XOR layer over the header. The encryption Steam applies to CM traffic is the channel encryption, and it lives one level up in `EnvelopeEncryptedConnection`, which wraps the whole `UdpConnection`. A reader looking for a "steamid"-style obfuscated UDP header will not find one here; the C# does not have one either.
- **`from_bytes` deviates from the C# constructor on purpose, in two ways.** (1) When the header cannot be parsed or the magic is wrong, the C# leaves `Payload` as a non-null but *empty* `MemoryStream`, which makes `IsValid` return **true** for a truncated datagram; the Rust port sets `payload = None`, so `is_valid()` is false and the datagram is dropped rather than dispatched as an empty message. (2) When `payload_size` exceeds `MAX_PAYLOAD`, the C# throws `ArgumentException` out of the constructor; the Rust port cannot throw from a constructor, so it drops the packet. Both deviations are documented in the source.
- **A short datagram is zero-padded.** The C# allocates `payload_size` bytes and reads whatever is available (`ReadAll` uses `ReadAtLeast` with `throwOnEndOfStream: false`). The Rust port reproduces this: it allocates `payload_size` bytes and copies only `min(available, length)` bytes, leaving the tail zeroed.
- **`from_bytes` overwrites `payload_size` and `msg_size`** with the payload length, matching the C# `GetPayloadAndUpdateHeader`.
- **`set_payload_slice` clamps rather than panicking.** If `offset + length` runs past the end of the source, the copy is clamped to the source bounds and the remainder of the payload is left zeroed — the same result the C# gets from reading past the end of a `MemoryStream`.
- **`payload()` returns an empty slice for an invalid packet** rather than an `Option`, so a caller that forgets to check `is_valid()` silently sees zero bytes. `UdpConnection` does check `is_valid()` before doing anything with a received packet.
- **The `UdpHeader`/`ChallengeData`/`ConnectData` types are duplicated, not shared.** The C# generates them from `netheader.steamd` into the `SteamKit2.Internal` namespace. The Rust port hand-writes them here because the `base` wave's protobuf generation does not cover that file; the module doc states they are byte-for-byte identical to the C# originals.

#### `crates/steamkit/src/networking/steam3/udp_connection.rs` — 727 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/UdpConnection.cs`
**Purpose:** Steam's reliable UDP transport: a sequenced, acknowledged, retransmitting datagram protocol with a challenge/connect/accept handshake. The state machine, the sequence numbering and the packet layout are ported verbatim from the C#; only the concurrency model changed (a dedicated `Thread` with a blocking `Socket` became a tokio task with an async `UdpSocket`).
**How it works:**

- `enum UdpState { Disconnected, ChallengeReqSent, ConnectSent, Connected, Disconnecting }` — the same five states as the C#.
- Constants: `RESEND_DELAY = 3 s`, `TIMEOUT_DELAY = 60 s`, `RESEND_COUNT = 3`, `AHEAD_COUNT = 5`, `OUTER_POLL = 150 ms`, `INNER_POLL = 10 ms`, `RECEIVE_BUFFER_SIZE = 2048`.
- `static SOURCE_CONN_ID: AtomicU32 = AtomicU32::new(512)` — a **process-wide** connection id, exactly like the C# `private static uint sourceConnId = 512`. `disconnect` advances it by 256, mirroring the C# comment "Advance this the same way that steam does, when a socket gets reused".
- `struct UdpStateData` holds the protocol bookkeeping: `remote_conn_id`, `out_seq` (starts at 1), `out_seq_sent`, `out_seq_acked`, `in_seq`, `in_seq_acked`, `in_seq_handled`, `out_packets: Vec<UdpPacket>` (the send queue), `in_packets: HashMap<u32, UdpPacket>` (the reassembly buffer), `time_out: Instant`, `next_resend: Instant`.
- `struct UdpShared { socket: Mutex<Option<Arc<UdpSocket>>>, current_end_point: Mutex<Option<SocketAddr>>, local_ip: Mutex<Option<IpAddr>>, state: tokio::sync::Mutex<UdpStateData>, disconnect_notified: AtomicBool, running: AtomicBool }`.
- `connect(end_point, _timeout)` binds a fresh `UdpSocket` to `0.0.0.0:0`, resets the whole `UdpStateData`, stores the endpoint and local IP, sets `running = true`, and spawns `udp_net_loop`. The `timeout` argument is **ignored** (named `_timeout`) — the C# ignores its `timeout` parameter too; the effective deadline is the 60-second `TIMEOUT_DELAY`.
- `udp_net_loop` sets the state to `ChallengeReqSent`, sends a `ChallengeReq` packet, and then runs the main loop:
  - an outer `recv_with_timeout(OUTER_POLL)` (150 ms); on a timeout it checks `time_out` and disconnects if it has expired; on a socket error it disconnects; on a datagram it calls `handle_received`.
  - if anything arrived, an inner drain with `INNER_POLL` (10 ms) processes the rest of the burst before the ack pass — the same reason the C# uses an inner 10 ms `Poll`: the more packets are processed together, the fewer separate acks are needed.
  - `send_pending_messages()` resends up to `RESEND_COUNT` packets when `now > next_resend && out_seq_sent > out_seq_acked` (clearing the queue first when the state is `Disconnecting`), and otherwise sends while `out_seq_sent < out_seq_acked + AHEAD_COUNT`.
  - an ack is sent as a `Datagram` packet whenever `in_seq != in_seq_acked` — the C# `SendAck`, used when there is no data to piggy-back the acknowledgement on.
  - a graceful shutdown is detected when the state is `Disconnecting` and the send queue is empty; the loop then sets `Disconnected`, records `user_requested_disconnect = true`, and breaks.
- `handle_received` drops any datagram whose sender is not the connected endpoint, refreshes `time_out`, parses the datagram with `UdpPacket::from_bytes`, and calls `receive_packet`.
- `receive_packet` rejects an invalid packet, rejects a packet whose `source_conn_id` does not match a known `remote_conn_id`, sends an ack and returns for a duplicate `Data` packet (`seq_this < in_seq`), processes the `seq_ack` (advance `out_seq_acked`, drop the acknowledged packets from `out_packets`, reset `next_resend`), advances `in_seq` as far as the reassembly buffer allows, and then dispatches by type: `Challenge → receive_challenge`, `Accept → receive_accept`, `Data → receive_data`, `Disconnect → state = Disconnected`, `Datagram → ignored`.
- `receive_challenge` (only in `ChallengeReqSent`) XORs the challenge with `ConnectData::CHALLENGE_MASK`, sends a sequenced `Connect` packet, and sets `in_seq_handled`.
- `receive_accept` (only in `ConnectSent`) records `remote_conn_id`, sets `in_seq_handled`, moves to `Connected` and raises the `Connected` event.
- `send(data)` splits the payload into `MAX_PAYLOAD`-sized `Data` packets, sets `header.msg_size` to the **total** length on each of them, and sends them as one sequenced message. The packet count is `total / MAX_PAYLOAD + 1`, which reproduces the C# `(ms.Length / UdpPacket.MAX_PAYLOAD) + 1` — including the extra, empty trailing packet when the length is an exact multiple of `MAX_PAYLOAD`.
- `send_sequenced` assigns `seq_this` from `out_seq`, `msg_start_seq` to the first sequence number of the message, `packets_in_msg` to the number of packets, pushes onto `out_packets`, and increments `out_seq`.
- `send_packet` stamps `source_conn_id` (the process-wide value), `dest_conn_id` (the remote one), and `seq_ack = in_seq` (also updating `in_seq_acked`), serializes with `get_data()`, and calls `socket.send_to`. A send failure sets the state to `Disconnected`.
- `ready_message_parts` and `dispatch_message` reassemble a multi-packet message: all `packets_in_msg` parts must be present before the message is emitted as one `NetMsgReceived` event.

**Notes and gotchas:**

- **The socket is rebound on every connect.** The C# binds its `Socket` in the constructor and keeps it for the object's lifetime, so Steam sees the same source port across reconnects and the source connection id is advanced by 256 to compensate. The Rust port binds a fresh `UdpSocket` to an ephemeral port on each `connect` and drops it when the loop ends, so the source *port* changes on every reconnect; only the connection *id* compensation (`SOURCE_CONN_ID += 256`) is preserved.
- **`SOURCE_CONN_ID` is shared by every UDP connection in the process.** Two `UdpConnection`s in the same process will stamp the same source connection id unless they are disconnected in between, and a disconnect by one advances the id for all of them. The C# has the same static.
- **The state mutex is a `tokio::sync::Mutex`** because it is held across `.await` (for example in `send_pending_messages` → `send_packet`); the socket and endpoint mutexes are `std::sync::Mutex` with statement-scoped guards.
- **The `Disconnected` event can be raised from two places.** `disconnect()` raises it immediately (with the caller's `user_initiated` flag), and the net loop raises it again when the graceful shutdown completes. The `disconnect_notified` swap guarantees exactly one event reaches the subscribers; the net loop's own raise is suppressed if `disconnect()` already won the race.
- **`send` silently drops data when the connection is not `Connected`.** Same as the C# (`if (state != State.Connected) return;`). A caller cannot tell a dropped send from a successful one.
- **The UDP transport performs no encryption of its own.** `default_connection_factory` in `steam/cm_client.rs` wraps the `UdpConnection` in an `EnvelopeEncryptedConnection`, exactly as it does for TCP, so CM traffic over UDP is channel-encrypted like the TCP path. The encryption sits above the datagram layer, not inside it.
- **Duplicate suppression uses the sequence number, not the payload.** A `Data` packet with `seq_this < in_seq` is dropped after re-acking; a packet with `seq_this <= in_seq_handled` or one already present in `in_packets` is dropped silently.
- **The `AHEAD_COUNT = 5` window limits in-flight packets** to five beyond the acknowledged sequence number, mirroring the C# comment "I've never seen Steam send more than 4 packets before it gets an Ack".
- **`challenge_value` is XORed with a fixed constant**, `0xA426DF2B`, which is the whole of the UDP handshake's "authentication". The `server_load` field of the challenge is parsed and ignored, as in the C#.
- **The C# `ReceiveData` uses `inPackets.TryAdd`**, which fails when the key already exists. The Rust port checks `contains_key` explicitly and returns; the observable behaviour is the same.
- **A `Disconnect` packet from the server sets the state to `Disconnected` but does not raise the event immediately** — the loop notices the state on its next iteration and raises `Disconnected` with `user_initiated = false`.

#### `crates/steamkit/src/networking/steam3/web_socket_context.rs` — 564 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/WebSocketContext.cs` (a nested class of `WebSocketConnection`)
**Purpose:** One websocket connection attempt: the URI, the read loop, the message reassembly and the send path. The socket implementation itself is isolated behind two traits so that the crate does not hard-depend on a websocket library, and the `tokio-tungstenite` implementation is provided alongside them.
**How it works:**

- Constants: `WEBSOCKET_PATH = "/cmsocket/"`, `WEBSOCKET_SCHEME = "wss"`, `READ_BUFFER_SIZE = 1024`.
- `enum WebSocketMessageType { Text, Binary, Close }` and `struct WebSocketReceiveResult { count: usize, end_of_message: bool, message_type: WebSocketMessageType }`.
- `trait WebSocketSocket` — the socket seam: `async send_binary(&self, data: Bytes) -> Result<(), SteamError>`, `async receive(&self, buffer: &mut [u8]) -> Result<WebSocketReceiveResult, SteamError>`, `async close(&self) -> Result<(), SteamError>`, `fn is_open(&self) -> bool`.
- `trait WebSocketFactory` — the connector seam: `async connect(&self, uri: &str, timeout: Duration) -> Result<Arc<dyn WebSocketSocket>, SteamError>`.
- `struct WebSocketContext { end_point: SocketAddr, uri: String, events: ConnectionEventSenders, cancel: CancellationToken, disconnect_notified: AtomicBool }`. `new(end_point, events)` derives the URI from the endpoint; `new_with_uri(end_point, uri, events)` takes it explicitly. `end_point()`, `uri()`, `is_cancelled()`, `cancel()`, `notify_disconnected(user_initiated)` (exactly-once, guarded by `disconnect_notified.swap`), and `start(socket)` which spawns the run loop.
- `run_loop` raises `Connected` first, then loops: it `select!`s (with `biased` ordering) on the cancellation token and on `socket.receive(&mut buffer)`. A `Binary` chunk is accumulated across `end_of_message` boundaries and emitted as one `NetMsgReceived` event; a `Text` chunk is logged; a `Close` triggers `notify_disconnected(false)` and returns. On exit the socket is closed and `notify_disconnected(false)` is called.
- `construct_uri(end_point)` builds `wss://{host}:{port}/cmsocket/`, bracketing IPv6 literals. `construct_uri_for_host(host, port)` is the same builder for a host *name* — the path the C# takes for a `DnsEndPoint`.
- `struct TungsteniteWebSocketSocket` wraps a `tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>`, split into a `Sink<Message>` and a `Stream<Item = Result<Message, WebSocketError>>` and each stored in its own `tokio::sync::Mutex<Option<..>>` (an `Option` so that `close` can take and drop them). `receive` pulls the next `Message` and maps `Binary` / `Text` / `Close` onto `WebSocketReceiveResult`; a `None` from the stream is `SteamError::Connection("websocket stream ended")`.
- `struct TungsteniteWebSocketFactory` implements `WebSocketFactory` with `tokio_tungstenite::connect_async(uri)` wrapped in `tokio::time::timeout`; a handshake failure is `SteamError::Connection`, a timeout is `SteamError::Timeout`. `ReqwestWebSocketFactory` is the configured-client variant: it sends the HTTP/1.1 upgrade through a `reqwest::Client`, validates `Sec-WebSocket-Accept`, and wraps the upgraded stream with `WebSocketStream::from_raw_socket`.

**Notes and gotchas:**

- **The `CMWebSocket` HTTP client purpose reaches this transport.** The C# drives a `ClientWebSocket` through an `HttpMessageInvoker`, so the user HTTP client - proxy, TLS, headers - applies to the CM websocket. The Rust port matches that with `ReqwestWebSocketFactory`: `default_connection_factory` calls `HttpClientFactory(HttpClientPurpose::CMWebSocket)` and performs the upgrade through that client. `TungsteniteWebSocketFactory` remains for callers that do not have a configured client.
- **The URI needs the host name, which `SocketAddr` does not carry.** The C# builds the URI from the `EndPoint`, and a `DnsEndPoint` keeps the host name — which is also the TLS server name. Because `IConnection::connect` is frozen to a `SocketAddr`, the host name has to be supplied out of band: `WebSocketConnection::new_tungstenite_for_host(host)` stores it and `construct_uri_for_host` uses it. Without it the URI would contain the resolved IP address and the TLS handshake would fail certificate validation.
- **The tungstenite implementation truncates binary messages larger than 1024 bytes.** `TungsteniteWebSocketSocket::receive` copies `min(payload.len(), buffer.len())` bytes into the caller's 1024-byte buffer and always reports `end_of_message: true`. The C# grows its output buffer when a message does not fit (`if (outputLength + result.Count > outputBuffer.Length)` → rent a double-sized buffer). A CM message larger than `READ_BUFFER_SIZE` is therefore truncated by this implementation, and the truncation is not detected. This is a real defect in the port and the first thing to fix if large messages are ever observed; a caller that needs the full message must implement `WebSocketSocket` itself.
- **The `WebSocketSocket`/`WebSocketFactory` traits are the seam that keeps the websocket library out of the core.** The module doc still describes the trait as letting "the crate compile without a websocket dependency", but `tokio-tungstenite` is in fact a hard dependency of this file; the traits remain useful as a substitution point for a different backend.
- **The C# reads in 1024-byte chunks and reassembles them; the Rust tungstenite socket hands over whole frames.** `tungstenite` already reassembles fragmented frames, so `end_of_message` is always `true` and the context's reassembly loop is a no-op for this implementation. The loop is still needed for a `WebSocketSocket` implementation that does report chunk boundaries.
- **A text message is logged and dropped**, exactly as in the C# — Steam's CM protocol is binary.
- **`notify_disconnected` is exactly-once.** The C# routes all disconnects through `connection.DisconnectCore(userInitiated, specificContext)`, which only raises the event if the context is still the connection's current one; the Rust context raises its own event unconditionally, and the `WebSocketConnection` decides whether to re-emit it.
- **The cancellation token is checked with `biased` ordering**, so a pending `cancel()` wins over a pending `receive`, which is what makes `disconnect` prompt.
- **`TungsteniteWebSocketSocket` sends `Message::Binary(data.to_vec().into())`**, which copies the `Bytes` into a `Vec`. The C# sends the `Memory<byte>` without a copy.

#### `crates/steamkit/src/networking/steam3/web_socket_connection.rs` — 229 lines
**C# counterpart:** `SteamKit2/SteamKit2/Networking/Steam3/WebSocketConnection.cs`
**Purpose:** The Steam CM websocket transport: the connection lifecycle (connect, disconnect, send) plus the event plumbing, with the socket itself behind the `WebSocketFactory` seam.
**How it works:**

- `struct WebSocketConnection { events: ConnectionEventSenders, factory: Arc<dyn WebSocketFactory>, host: Option<String>, current: Arc<Mutex<Option<CurrentConnection>>>, current_end_point: Arc<Mutex<Option<SocketAddr>>> }` with `struct CurrentConnection { context: Arc<WebSocketContext>, socket: Arc<dyn WebSocketSocket> }`.
- Constructors: `new(factory)`, `new_for_host(factory, host)` (the variant that preserves the TLS/URI host name), `new_tungstenite()`, `new_tungstenite_for_host(host)`. `default_connection_factory` uses `new_for_host` with a `ReqwestWebSocketFactory`.
- `connect(end_point, timeout)` builds the URI — `construct_uri_for_host(host, end_point.port())` when a host name was supplied, `construct_uri(end_point)` otherwise — and calls `factory.connect(&uri, timeout)`. On failure it logs `Exception connecting websocket: {err}`, raises `Disconnected(false)` and returns the error. On success it creates the context, swaps the new connection into `current`, and — if there was an old one — logs `Attempted to connect while already connected. Closing old connection...`, cancels the old context, closes the old socket, and raises `Disconnected(false)`. It then stores the endpoint and starts the context.
- `disconnect(user_initiated)` takes the current connection out of `current`, cancels the context, calls `context.notify_disconnected(user_initiated)`, closes the socket, and clears the endpoint.
- `send(data)` returns `Ok(())` when there is no current connection; otherwise it calls `socket.send_binary(data)`, and on error logs `Exception while sending data: {err}`, calls `self.disconnect(false)` and returns the error.
- `protocol_types()` returns `ProtocolTypes::WEB_SOCKET`. `local_ip()` returns `Some(255.255.255.255)` — the C# returns `IPAddress.None`, which **is** `255.255.255.255` and not null.

**Notes and gotchas:**

- **The handshake is eager here and lazy in the C#.** The C# `Connect` creates the context, swaps it in and returns; the websocket handshake happens inside the context's run loop, and a failed handshake surfaces as a `Disconnected` event rather than as an exception. The Rust port performs the handshake inside `connect` and returns `Err` — and *also* raises `Disconnected(false)` on failure, so an event-driven caller still observes the disconnection. A caller that only awaits `connect` now gets an `Err` where the C# would have returned normally and signalled the failure through the event.
- **The C# `WebSocketConnection` is `IDisposable` and disposes the `HttpMessageInvoker`** it was constructed with. The Rust port owns a `WebSocketFactory`, not an HTTP client, and has no `Drop` implementation; the socket is closed explicitly by `disconnect`.
- **The `Connected` event comes from the context, not from `connect`.** `connect` returns as soon as the socket is up; `WebSocketContext::run_loop` raises `Connected` at its start, on the spawned task.
- **`local_ip()` returns `Some(255.255.255.255)`, not `None`.** The C# `WebSocketConnection.GetLocalIP` returns `IPAddress.None`, which is `255.255.255.255` and **not** null — the C# sends the logon with that address. The Rust port used to return `None`, and the difference mattered: with a null local address the logon fails *before* the `ClientLogon` is ever sent. It now returns `Some(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)))`, so the websocket transport reports the same address the C# reports (an earlier fix returned `0.0.0.0`, which obfuscates to a different `obfuscated_private_ip`; the porting notes §13). `SteamClient::local_ip()` therefore yields `255.255.255.255` for a websocket connection, where the TCP and UDP transports report a real bound address.
- **The websocket transport completes a logon, but the session does not last.** The `local_ip` fix above is what lets the logon be sent at all; with it, the server answers the logon exactly as it answers the C# reference over the same transport. What the server then does is close the websocket CM session after a minute or two, while a TCP session stays up. That is why `default_connection_factory` prefers TCP (see the `cm_client.rs` entry and the porting notes §13).
- **The host name is per-connection, not per-factory.** The `host` field is set at construction time; there is no way to change it for a later reconnect. A `CMClient` that reconnects to a different CM host therefore needs a new `WebSocketConnection` — which is what `default_connection_factory` does, since it builds a fresh connection per `connect` call and receives the `ServerRecord` (and thus the host) as an argument.
- **The C#'s `DisconnectCore` distinguishes "the context that failed" from "any context".** The Rust port has no such distinction: `disconnect` always tears down whatever is current, and the context raises its own `Disconnected` event once.

#### `crates/steamkit/src/steam/steam_client/` — the client facade, jobs, callbacks and configuration

This directory is the Rust port of the C# `SteamKit2/Steam/SteamClient/` namespace. It holds the public face of the library and the three subsystems that hang off it:

- `steam_client.rs` — the `SteamClient` facade. The C# `SteamClient` derives from `CMClient`; Rust has no inheritance, so the Rust `SteamClient` **owns** an `Arc<CMClient>` and forwards the inherited surface to it explicitly. It also owns the handler registry, the callback queue, the job manager and the callback manager.
- `async_job_manager.rs` (+ `async_job_failed_exception.rs`) — the registry of in-flight requests, the counterpart of the C# `AsyncJobManager` plus the `AsyncJob`/`AsyncJob<T>`/`AsyncJobMultiple<T>` classes that live in `Types/JobID.cs`. The awaitable job *types* themselves live in `crate::types::job_id`; this module owns the *registry* and re-exports the types.
- `callback_mgr/` — the `CallbackManager` and the `CallbackMsg` trait: the machinery that routes a posted callback to the functions that subscribed to it.
- `configuration/` — `SteamConfiguration`, its builder, its state struct and the machine-info provider seam.
- `callbacks.rs` — the two lifecycle callbacks the client posts for itself.

Three properties hold across the whole directory and are worth stating once:

1. **No panics.** Every fallible path returns `SteamError`. The C# throws `InvalidOperationException` in a few places (`GetRequiredHandler`, `AddHandler` on a duplicate, `ArgumentNullException` on a null argument); the Rust port returns an `Err` instead.
2. **No `unsafe`.** The crate is `#![forbid(unsafe_code)]`.
3. **The handler-facing API is frozen.** `SUMMARY.md` §8 fixes the surface that all fifteen handler sections code against. Two signatures in the shipped code deviate from that freeze in the direction of *not* panicking (`get_required_handler` returns a `Result`); they are called out in the entries below.

#### `crates/steamkit/src/steam/steam_client/mod.rs` — 6 lines
**C# counterpart:** none — the C# `SteamKit2` namespace is flat and has no per-directory manifest; this file is the Rust module list.
**Purpose:** Declares the six submodules of the `steam_client` tree.
**How it works:** `pub mod async_job_failed_exception; pub mod async_job_manager; pub mod callback_mgr; pub mod callbacks; pub mod configuration; pub mod steam_client;`
**Notes and gotchas:** The C# puts `CallbackMgr` and `Configuration` in sub-namespaces (`SteamKit2` still, but in separate folders), and the Rust port mirrors the folder layout with nested modules. The public types are re-exported at the crate root (`steamkit::SteamClient`), so most consumers never name these module paths.

#### `crates/steamkit/src/steam/steam_client/steam_client.rs` — 725 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/SteamClient.cs` (a `partial class SteamClient : CMClient`)
**Purpose:** The public entry point of the library: it owns the connection (through a `CMClient`), the registry of client-message handlers, the callback queue, the job manager and the callback manager, and it exposes the frozen handler-facing API.
**How it works:**

- `pub struct SteamClient { cm: Arc<CMClient>, weak: Weak<SteamClient>, handlers: RwLock<Vec<HandlerEntry>>, current_job_id: AtomicU32, process_start_time: u32, callback_queue: Mutex<VecDeque<Box<dyn CallbackMsg>>>, callback_notify: tokio::sync::Notify, job_manager: AsyncJobManager, callback_manager: CallbackManager, authentication: std::sync::OnceLock<SteamAuthentication> }`.
- `struct HandlerEntry { type_id: Option<TypeId>, handler: Arc<dyn ClientMsgHandler>, any: Option<Arc<dyn Any + Send + Sync>> }` — one entry per registered handler. `type_id` and `any` are populated only for handlers that were registered with their concrete type known.
- Construction goes through `Arc::new_cyclic`, which produces the `Weak<SteamClient>` self-reference that the handlers and the dispatcher need. `SteamClientDispatcher` implements `ClientMsgDispatcher` and upgrades the weak reference to route `on_client_msg_received`, `on_client_connected` and `on_client_disconnected` back into the client.
- Constructors: `new()`, `with_config(config)`, `with_identifier(identifier)`, `with_configuration(Arc<SteamConfiguration>)`, `with_configuration_and_identifier(configuration, identifier)`. An empty identifier panics with "identifier must not be empty", the `ArgumentException` equivalent; `with_configuration` supplies `random_identifier()` (a 32-character hex string) itself. The constructor also kicks off `HardwareUtils::init(configuration.machine_info_provider().clone())` so that the machine info is ready by logon time - the C# does the same with `HardwareUtils.Init`.
- The forwarding surface (everything the C# inherited from `CMClient`): `configuration`, `identifier`, `id`, `servers`, `local_ip`, `current_end_point`, `public_ip`, `ip_country_code`, `user_country_code`, `universe`, `is_connected`, `session_token`, `cell_id`, `session_id`, `steam_id`, `connection_timeout`, `debug_network_listener`, `set_debug_network_listener`, `expect_disconnection`, `set_expect_disconnection`, `cm_client`.
- The handler registry:
  - `add_handler(&self, handler: Arc<dyn ClientMsgHandler>)` rejects a duplicate **by pointer identity** (`Arc::ptr_eq`) with a `tracing::warn!` and keeps the existing handler.
  - `add_handler::<T>(&self, handler: T) -> Result<(), SteamError>` records the `TypeId` and keeps the concrete `Arc<T>` in `any`, so the handler stays retrievable. A second handler of the same type yields `SteamError::InvalidArgument`.
  - `remove_handler(&self, handler: &Arc<dyn ClientMsgHandler>) -> bool` removes by pointer identity; `remove_handler_type::<T>() -> bool` removes by type.
  - `get_handler::<T>() -> Option<Arc<T>>` finds the entry with a matching `TypeId` and downcasts it.
  - `get_required_handler::<T>() -> Result<Arc<T>, SteamError>` returns `SteamError::NotFound` when the handler is not registered.
- The callback queue is an unbounded `mpsc` channel: `get_callback()` (a non-blocking `try_lock` + `try_recv`), `wait_for_callback()` (async, holds the receiver lock while waiting), `wait_for_callback_timeout(timeout)`, `wait_for_callback_async(cancellation)`.
- `post_callback(msg)` queues the message first and then completes the job (the C# order), requesting a disconnect when the job manager reports a callback-type mismatch; `post_callback_typed::<T>(msg)` is the typed convenience form.
- `run_callbacks()` takes one queued callback and dispatches it (spawning async handlers); `run_wait_callbacks()`, `run_wait_callbacks_timeout(timeout)` and `run_wait_all_callbacks(timeout)` await the async handlers through `dispatch_async`.
- `subscribe::<T>(f)`, `subscribe_guarded::<T>(f)` (returns a `SubscriptionGuard` that unsubscribes on drop), `subscribe_job::<T>(job_id, f)` and `unsubscribe(id)` delegate to the `CallbackManager`.
- `get_next_job_id()` builds the job id: `sequence = current_job_id.fetch_add(1) & 0xF_FFFF`, `value = (process_start_time as u64) << 20 | sequence`. `capture_process_start_time()` stores `(unix_seconds - 1_104_537_600) & 0x3FFF_FFFF`, where `unix_seconds` is the OS process start time read once per process through PowerShell (`Get-Process -Id <pid>`), falling back to the first-use time.
- `start_job::<T>(job_id)` registers the job with the job manager and fails it immediately when the client is not connected.
- `connect(cm_server: Option<ServerRecord>)` is synchronous and fire-and-forget: it rejects an already-connected client, spawns the `CMClient` connect on the current runtime, and reports failures as a `DisconnectedCallback`; `disconnect()` delegates to the `CMClient`.
- `on_client_msg_received(packet)` handles `EMsg::JobHeartbeat` and `EMsg::DestJobFailed` before the handlers, then iterates a snapshot. A handler that returns an error is logged, the client is disconnected with `user_initiated = false`, and the function returns `false`; a pending disconnect request from a callback-type mismatch is consumed after the handler loop.
- `on_client_connected()` enables the job timeouts and posts a `ConnectedCallback`. `on_client_disconnected(user_initiated)` posts a `DisconnectedCallback`, cancels all pending jobs, disables the job timeouts, and calls `clear_handler_caches()` (which clears the `SteamMatchmaking` lobby cache).
- `add_default_handlers` installs the fifteen handlers the library ships with, in the C# order: `SteamFriends`, `SteamUser`, `SteamApps`, `SteamGameCoordinator`, `SteamGameServer`, `SteamUserStats`, `SteamMasterServer`, `SteamCloud`, `SteamWorkshop`, `SteamUnifiedMessages`, `SteamScreenshots`, `SteamMatchmaking`, `SteamNetworking`, `SteamContent`, `SteamAuthTicket`. `SteamFriends` must come before `SteamUser` because of the `AccountInfoCallback` dependency.

**Notes and gotchas:**

- **The C# `SteamClient` inherits roughly forty public members from `CMClient`; the Rust port forwards them by hand.** Anything the C# exposed on `CMClient` that the port did not forward is only reachable through `cm_client()`. When adding a member to `CMClient`, check whether it also needs a forwarding method here.
- **`get_required_handler` returns a `Result`, not `Arc<T>`.** `SUMMARY.md` §8 froze the signature as `pub fn get_required_handler<T>(&self) -> Arc<T>`, but the C# throws `InvalidOperationException` there, and the crate forbids panicking in library code. The shipped signature is `Result<Arc<T>, SteamError>` with `SteamError::NotFound`. Handlers that need the handler must add `?` or `.expect(...)` at the call site.
- **`add_handler` rejects a duplicate *type*.** It is generic over the handler type and records `TypeId::of::<T>()`, so a second handler of the same type returns `SteamError::InvalidArgument`, matching the C# `GetType()` comparison and `InvalidOperationException`.
- **`post_callback` queues before completing the job.** That is the C# order (`callbackQueue.Post(msg)` then `jobManager.TryCompleteJob(...)`), so a waiter that wakes up can already see the queued callback.
- **Only one task can wait on the callback queue at a time.** `wait_for_callback` holds the receiver's `tokio::sync::Mutex` for the whole wait, so a second concurrent waiter blocks on the mutex rather than receiving a message of its own. The C#'s `BufferBlock<CallbackMsg>.Receive` supports multiple receivers. `get_callback` uses `try_lock` and returns `None` when another task currently holds the lock, so a poll loop can spuriously observe "no callback" while another task is waiting.
- **`get_next_job_id` masks the sequence to 20 bits** (`& 0xF_FFFF`) and places it in the low 20 bits of the `JobID`, with the process start time above it. The C# builds the same value through the `JobID` property setters, which truncate to the field widths.
- **`authentication()` is lazily initialised** through a `std::sync::OnceLock<SteamAuthentication>`; the helper receives a `Weak<SteamClient>` so that the client and the handlers do not form an `Arc` cycle.
- **The handler list is snapshotted before dispatch**, so a handler that registers or removes a handler while processing a message does not disturb the iteration — but the new handler will not see the current message either.
- **`receive_test_packet_msg`** is the counterpart of the C# `SteamClient.ReceiveTestPacketMsg` test hook: it feeds a packet straight into the handler pipeline without a live connection, and returns `true` when every handler accepted it.
- **A handler error disconnects the client.** The C# catches `ProtoException` and any other exception from `HandleMsg`, logs, disconnects with `user_initiated: false` and returns `false`. The Rust port does the same for any `Err` returned by `handle_msg`; the protobuf-specific distinction is lost because the handler API returns a single error type.

#### `crates/steamkit/src/steam/steam_client/async_job_manager.rs` — 513 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/AsyncJobManager.cs` (together with the `AsyncJob`, `AsyncJob<T>` and `AsyncJobMultiple<T>` classes that live in `SteamKit2/Types/JobID.cs`)
**Purpose:** The registry of in-flight requests. It registers a job before the request is sent, completes it when the matching response arrives, keeps it alive on heartbeats, fails it on a remote failure, and cancels it on disconnect or timeout.
**How it works:**

- `pub struct AsyncJobManager { inner: Arc<AsyncJobManagerInner>, timeouts_enabled: AtomicBool, timeout_task: Mutex<Option<JoinHandle<()>>> }` with `struct AsyncJobManagerInner { jobs: DashMap<JobID, Arc<JobEntry>> }`. The `DashMap` is the counterpart of the C# `ConcurrentDictionary<JobID, AsyncJob>`.
- `struct JobEntry { type_id: TypeId, clone_callback: fn(&dyn CallbackMsg) -> Option<Box<dyn CallbackMsg>>, completer: Completer, timeout: Mutex<Duration>, started: Instant, finish_condition: Mutex<Option<Arc<dyn Fn(&dyn CallbackMsg) -> bool + Send + Sync>>>, results: Mutex<Vec<Box<dyn CallbackMsg>>> }`.
- `enum Completer { Single(Mutex<Option<oneshot::Sender<Result<Box<dyn CallbackMsg>, SteamError>>>>), Multiple(Mutex<Option<MultiCompleter>>) }` — the single-result and multi-result completion channels. `MultiCompleter` is `Box<dyn FnOnce(MultiOutcome) + Send + Sync>` and `MultiOutcome` is either `Results { results, complete, failed }` or `Error(SteamError)`. The `Option` inside the `Mutex` is taken on completion, so a second completion attempt is a no-op.
- `start_job::<T>(job_id) -> AsyncJob<T>` builds the `AsyncJob`, takes its `oneshot::Sender` out, and registers a `JobEntry` with the expected `TypeId` and a `clone_callback` specialised to `T`.
- `start_job_multiple::<T>(job_id, finish_condition) -> AsyncJobMultiple<T>` shares **one** `Arc<Mutex<Box<dyn FnMut(&T) -> bool>>>` between the manager-side predicate (which downcasts the erased callback) and the job-side predicate, so both observe the same state.
- `try_complete_job(job_id, callback) -> bool`:
  - unknown job: returns `false`.
  - single-result job: a `TypeId` mismatch fails the job with `SteamError::Malformed` and returns `true` (the caller disconnects); otherwise the callback is cloned into the `oneshot` and the job is removed.
  - multi-result job: evaluates the finish condition, accumulates the cloned callback, completes when the condition is met, otherwise heartbeats; returns `false`.
- `heartbeat_job(job_id)` extends the job's timeout by `JOB_HEARTBEAT_EXTENSION` (10 s).
- `fail_job(job_id)` removes the job; with no accumulated results it fails the job with `SteamError::JobFailed`, with results it completes the multi-job with `complete = false, failed = true`.
- `cancel_pending_jobs()` clears the map and fails every job: `SteamError::Cancelled` for a job with no results, a `ResultSet { complete: false, failed: false }` for one with results.
- `cancel_timed_out_jobs()` removes the timed-out jobs and fails them with `SteamError::Timeout(entry.timeout())` (or completes a multi-job with partial results).
- `set_timeouts_enabled(enable)` starts or stops the periodic sweep. The sweep is a tokio task on a `JOB_TIMEOUT_CHECK_INTERVAL` (1 s) interval with `MissedTickBehavior::Delay`; it holds a `Weak<AsyncJobManagerInner>` and stops when the manager is dropped.
- `tracked_job_count()` and `timeouts_enabled()` are introspection helpers.
- The module re-exports the job types from `crate::types::job_id`: `AsyncJob`, `AsyncJobMultiple`, `JobHandle`, `JobReceiver`, `MultiJobReceiver`, `ResultSet`, `TypedJobReceiver`, plus the local aliases `MultiJobHandle<T> = AsyncJobMultiple<T>` and `JobResultSet<T> = ResultSet<T>`.

**Notes and gotchas:**

- **The C# `AsyncJobManager` is `internal`; the Rust port is `pub`.** The C# keeps it private to the assembly; the Rust port exposes it so that the handler sections can await their own requests through `SteamClient::job_manager()`.
- **The awaitable job types live elsewhere.** `crate::types::job_id` owns `AsyncJob<T>`, `AsyncJobMultiple<T>`, `JobReceiver`, `TypedJobReceiver<T>`, `MultiJobReceiver<T>` and `ResultSet<T>`, and this module only owns the *registry*. The naming aliases live in `types::job_id` as well: `pub type JobState = AsyncJob;` (the untyped marker job) and `pub type JobHandle<T> = AsyncJob<T>;` (the typed single-result job). `JobHandle`/`JobState` are aliases only — the canonical names remain `AsyncJob*`, so a doc comment or an error message that says "job handle" refers to `AsyncJob<T>`.
- **The callback is cloned, not shared.** The C# hands the *same* `CallbackMsg` instance to the job and to the callback queue (`callbackQueue.Post(msg); jobManager.TryCompleteJob(msg.JobID, msg);`). A Rust `Box<dyn CallbackMsg>` cannot be owned twice, so the manager clones the concrete callback via the `clone_callback` function pointer, which is only possible because `T: CallbackMsg + Clone` is a bound on `start_job`. The clone is made only when the callback's type matches the job's expected type.
- **A type mismatch disconnects, like the C#.** The C# cast throws `InvalidCastException`, which bubbles through `PostCallback` and disconnects the client. The Rust `try_complete_job` fails the job with `SteamError::Malformed`, returns `true`, and `SteamClient::post_callback` sets the disconnect request that the message loop consumes.
- **A timeout is `SteamError::Timeout`, not a cancellation.** The C# maps both a timeout and a cancel to `TaskCompletionSource.TrySetCanceled()` (`SetFailed(dueToRemoteFailure: false)`), so a timed-out C# job surfaces as a *cancellation*. The Rust port distinguishes them: `cancel_pending_jobs` yields `SteamError::Cancelled` and `cancel_timed_out_jobs` yields `SteamError::Timeout(duration)`. Code that matched on cancellation in C# must match on `Timeout` here.
- **A remote failure is `SteamError::JobFailed`,** the counterpart of the C# `AsyncJobFailedException` (which the C# raises through `TrySetException`). `SteamError::JobFailed` is a unit variant — it carries no message, unlike the C# exception.
- **The timeout sweep needs a tokio runtime.** `start_timeout_task` bails out silently if `tokio::runtime::Handle::try_current()` fails. A client created outside a runtime therefore never times jobs out — the jobs simply live until they are completed or the client disconnects. The C#'s `ScheduledFunction` works without an explicit runtime.
- **The timeout is shared between the job and the manager.** `AsyncJob::timeout` and `set_timeout` use an `Arc<Mutex<Duration>>` that the manager `JobEntry` holds too, so changing it on the handle changes the deadline the manager enforces.
- **The `oneshot` sender is consumed on completion.** `Completer::complete_single` and `Completer::fail` both `take()` the sender out of its `Option`, so a job can only ever be completed once even if `try_complete_job` is called again with the same job id.
- **The job manager is public and `Send + Sync`**, so `Arc<AsyncJobManager>` can be shared; the `DashMap` allows concurrent `try_complete_job` calls from several tasks.
- **`DEFAULT_JOB_TIMEOUT` is 10 seconds**, matching the C# `AsyncJob.Timeout` default of `TimeSpan.FromSeconds(10)`.

#### `crates/steamkit/src/steam/steam_client/async_job_failed_exception.rs` — 74 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/AsyncJobFailedException.cs`
**Purpose:** The error raised when Steam rejects a pending asynchronous job.
**How it works:** `pub struct AsyncJobFailedException { message: String, source: Option<Box<dyn Error + Send + Sync>> }` with `new()` (the default message "The job failed remotely."), `with_message(message)`, `with_cause(message, cause)` and a `message()` accessor. It implements `Display` (message, plus `: {source}` when a cause is present), `Error` (with `source()`), `Default`, and `From<AsyncJobFailedException> for SteamError`, which maps it to the unit variant `SteamError::JobFailed`.
**Notes and gotchas:** In C# this is an exception that escapes through `TaskCompletionSource.TrySetException`. In Rust the crate has exactly one public error type, so the type is a *value* that converts into `SteamError` — but the conversion discards the message and the cause, so `SteamError::JobFailed` carries no detail. In practice the job manager never constructs this type: `fail_job` produces `SteamError::JobFailed` directly. The type exists so that the C#'s exception shape is still expressible, and so that `AsyncJobFailedException` can be named in the porting notes and in the error model. Note that the C# class is `public` and derives from `Exception`; the Rust type is not `Clone` and cannot be `Send` without the `source` field being `Send`, which the `Box<dyn Error + Send + Sync>` bound guarantees.

#### `crates/steamkit/src/steam/steam_client/callback_mgr/mod.rs` — 3 lines
**C# counterpart:** none — the C# puts `CallbackMgr` in a folder but in the same `SteamKit2` namespace, with no namespace declaration of its own; this file is the Rust module list.
**Purpose:** Declares the three submodules of the callback manager.
**How it works:** `pub mod callback_mgr; pub mod callback_mgr_internals; pub mod callback_msg;`
**Notes and gotchas:** The C#'s `CallbackMgr_Internals.cs` lives in the `SteamKit2.Internal` namespace (it declares `internal` classes), while `CallbackMgr.cs` and `CallbackMsg.cs` are in `SteamKit2`. The Rust port puts all three in one module tree and makes the internals public, because the callback manager's public API returns and accepts them.

#### `crates/steamkit/src/steam/steam_client/callback_mgr/callback_msg.rs` — 74 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/CallbackMgr/CallbackMsg.cs`
**Purpose:** The base of every callback the client posts. It carries the job id the callback refers to and lets the dispatcher recover the concrete type.
**How it works:** `pub trait CallbackMsg: Any + Send + Sync` with six methods: `job_id`, `set_job_id`, `as_any`, `as_any_mut`, `as_a(type_id) -> Option<&dyn Any>` and `is_a(type_id) -> bool`. `as_a` defaults to an exact match; a hierarchy can override it, and `is_a` follows `as_a`. `pub struct CallbackMsgBase { job_id: JobID }` provides the storage: `new()` (job id `JobID::INVALID`), `with_job_id(job_id)`, `job_id()`, `set_job_id(id)`. `CallbackMsgBase` is `Clone + Copy + Debug + PartialEq + Eq + Hash`.
**Notes and gotchas:** The C# `CallbackMsg` is an abstract class with an auto-property `JobID` that is initialised to `JobID.Invalid` in the constructor. Rust has no class inheritance, so the trait carries the four operations and `CallbackMsgBase` is a mixin that implementors embed. A callback struct typically looks like `struct MyCallback { base: CallbackMsgBase, ... }` and forwards the four trait methods to `self.base`, exactly as `callbacks.rs` does. `JobID::INVALID` is `u64::MAX` in the Rust port, the counterpart of the C# `JobID.Invalid`. The `Any` supertrait is what makes the type-based dispatch in `CallbackManager` possible; `as_any`/`as_any_mut` are the explicit escape hatches, because `Any` cannot be used directly on a trait object without them.

#### `crates/steamkit/src/steam/steam_client/callback_mgr/callback_mgr_internals.rs` — 192 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/CallbackMgr/CallbackMgr_Internals.cs` (the C# `CallbackBase`, `Callback<TCall>` and `AsyncCallback<TCall>` classes, in the `SteamKit2.Internal` namespace)
**Purpose:** The bookkeeping behind a registered callback subscription: the type it listens for, the job it is restricted to, and the handler that is invoked.
**How it works:**

- `pub enum CallbackHandler { Sync(Box<dyn Fn(&dyn CallbackMsg) + Send + Sync>), Async(Box<dyn Fn(Arc<dyn Any + Send + Sync>) -> BoxFuture<'static, ()> + Send + Sync>) }` — the two handler shapes, erased.
- `pub struct CallbackSubscription { type_id: TypeId, job_id: Option<JobID>, handler: CallbackHandler }` with `new(...)`, `callback_type()`, `job_id()`, `handler()`, and `matches(msg: &dyn CallbackMsg, job_id)`. `matches` asks `msg.is_a(self.type_id)` (exact by default, base view when the callback overrides `as_a`), then applies the job filter: no filter means all callbacks of that type; `Some(id)` matches when `id == JobID::INVALID` or `id == job_id`.
- `pub struct Callback<TCall> { job_id: Option<JobID>, on_run: Box<dyn Fn(&TCall) + Send + Sync> }` — the typed synchronous subscription. `into_subscription()` erases it: it captures the `TypeId` of `TCall` and wraps the typed closure in a `CallbackHandler::Sync` that downcasts the erased message with `msg.as_any().downcast_ref::<TCall>()`.
- `pub struct AsyncCallback<TCall> { job_id: Option<JobID>, on_run: Box<dyn Fn(Arc<TCall>) -> BoxFuture<'static, ()> + Send + Sync> }` — the typed asynchronous subscription. `into_subscription()` wraps the closure in a `CallbackHandler::Async` that downcasts the shared `Arc<dyn Any + Send + Sync>` with `msg.downcast::<TCall>()`; a failed downcast produces an empty future.

**Notes and gotchas:**

- **The dispatch filter goes through `CallbackMsg::is_a`, not raw `TypeId` equality.** The default is an exact match; a callback that models a hierarchy overrides `as_a`, so a base subscription receives a derived callback, like the C# `CallbackType.IsAssignableFrom(type)`.
- **The C# subscriptions are self-registering `IDisposable` objects.** `Callback<TCall>` registers itself with the manager in its constructor and unregisters in `Dispose` (and in its finalizer). The Rust `Callback<TCall>` / `AsyncCallback<TCall>` are inert values: registration is `CallbackManager::register` and removal is `unsubscribe(id)` or dropping a `SubscriptionGuard` from `subscribe_guarded`.
- **The async handler receives an owned `Arc`, the sync handler a reference.** The C# `AsyncCallback<TCall>` receives a `TCall` by value (`Func<TCall, Task>`) and the sync one a `TCall` by value as well (`Action<TCall>`). The Rust port passes `&TCall` to the sync handler and `Arc<TCall>` to the async handler, so an async handler can hold the callback across an `.await`.
- **The job filter semantics match the C#.** The C# `CallbackBase.Run` checks `cb.JobID == JobID || JobID == JobID.Invalid`; the Rust `matches` reproduces this, including the `JobID::INVALID` sentinel meaning "every job".
- **The `matches` check is done once per dispatch, outside the handler.** The typed handler still goes through `as_a` before downcasting, so a base subscription receives the base view rather than the derived value.

#### `crates/steamkit/src/steam/steam_client/callback_mgr/callback_mgr.rs` — 325 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/CallbackMgr/CallbackMgr.cs`
**Purpose:** Routes a posted callback to every subscription that is interested in it.
**How it works:**

- `pub struct CallbackManager { subscriptions: RwLock<Vec<(SubscriptionId, Arc<CallbackSubscription>)>>, next_id: AtomicU64, unified_messages: OnceLock<Weak<SteamUnifiedMessages>> }` — a `parking_lot::RwLock` over a vector of `(id, subscription)` pairs, with a monotonically increasing id source. The `OnceLock` holds the back-reference that `SteamClient` sets after registering the default handlers, so `subscribe_service_*` can create the service.
- `pub struct SubscriptionId(u64)` — a `Copy` handle with a `value()` accessor.
- `pub struct SubscriptionGuard<'a>` — an RAII handle returned by `subscribe_guarded`; it borrows the manager and calls `unsubscribe` on drop, the Rust form of a C# `using` scope.
- Subscription methods, all returning a `SubscriptionId`: `subscribe::<T>(f)`, `subscribe_job::<T>(job_id, f)`, `subscribe_async::<T>(f)`, `subscribe_job_async::<T>(job_id, f)`, `subscribe_service_notification::<T>(f)`, `subscribe_service_notification_async::<T>(f)`, `subscribe_service_response::<T>(f)`, `subscribe_service_response_async::<T>(f)`. `subscribe_guarded::<T>(f)` is the exception: it returns a `SubscriptionGuard<'_>`. The service variants create the service and register the body type before registering the callback.
- `unsubscribe(id)` removes the entry; `subscription_count()` returns the number of live subscriptions.
- `dispatch(msg: Box<dyn CallbackMsg>)` wraps the message in an `Arc` and calls `dispatch_arc`. `dispatch_arc(msg: Arc<dyn CallbackMsg>)` takes a snapshot of the subscription list, and for each subscription whose `matches(msg, job_id)` accepts the callback (through `CallbackMsg::is_a`) invokes the handler: a `Sync` handler is called inline, an `Async` handler is spawned onto the current tokio runtime.
- `dispatch_async(msg)` / `dispatch_arc_async(msg)` do the same but `await` the async handlers one at a time instead of spawning them.
- `register(subscription)` allocates the id with `next_id.fetch_add(1, Ordering::Relaxed)` and pushes the entry onto the vector.

**Notes and gotchas:**

- **The Rust manager is standalone; the C# manager owns the client.** The C# `CallbackManager` holds a `SteamClient` and *pulls* callbacks out of the client's queue itself (`RunCallbacks` → `client.GetCallback()` → `Handle(call)`); it also resolves `client.GetHandler<SteamUnifiedMessages>()` in its constructor. The Rust manager is a pure router: the `SteamClient` owns the queue and calls `callback_manager.dispatch(...)` explicitly, which is why `SteamClient::run_callbacks` and friends exist.
- **Dropping a `SubscriptionId` does not unsubscribe; dropping a `SubscriptionGuard` does.** The C# `Subscribe` returns an `IDisposable`, and only an explicit `Dispose` removes the subscription — dropping the reference does not. The port mirrors that: `subscribe` returns the `Copy` id and requires `unsubscribe(id)`, while `subscribe_guarded` returns a guard whose `Drop` calls it, like `using var sub = ...`. A `Drop` on the id itself would silently unsubscribe statement-level `subscribe(...)` calls, which the C# does not do.
- **`dispatch` does not await async handlers.** `CallbackHandler::Async` handlers are spawned onto the current runtime via `tokio::runtime::Handle::try_current()`; **if no runtime is current the async handler is silently skipped**. The C# `Handle` blocks on the task (`if (!task.IsCompletedSuccessfully) task.AsTask().Wait();`), so `RunCallbacks` waits for async callbacks to finish. `dispatch_async` is the Rust counterpart of `RunWaitCallbackAsync` and does await them; `SteamClient::run_wait_callbacks`, `run_wait_callbacks_timeout` and `run_wait_all_callbacks` use it.
- **The subscription order is preserved.** The C# keeps the subscriptions in an `ImmutableList` and iterates them in registration order; the Rust port keeps them in a `Vec` and iterates the snapshot in the same order. This is *not* the same as the C# `event` ordering caveat that applies to the connection events: within one callback dispatch, the handlers still run in registration order.
- **The snapshot makes re-entrant subscription safe.** A handler that subscribes or unsubscribes during a dispatch does not invalidate the iteration; the change takes effect from the next dispatch.
- **The service subscriptions create the service.** In the C#, `SubscribeServiceNotification<TService, TNotification>` and `SubscribeServiceResponse<TService, TResponse>` call `steamUnifiedMessages.CreateService<TService>()` before registering. The Rust variants do the same through the manager's `unified_messages` back-reference and register the body type on the service; they identify the service by name rather than by a type parameter.
- **The type filter goes through `CallbackMsg::is_a`.** The default is an exact `TypeId` match; a callback that models a hierarchy can override `as_a` so a base subscription receives it, like the C# `IsAssignableFrom`.
- **The manager does not hold a strong reference to the client**, so it cannot keep a dropped client alive; the `Arc<CallbackSubscription>` values are shared with the snapshot, which is why a handler may be running while it is being unsubscribed.

#### `crates/steamkit/src/steam/steam_client/callbacks.rs` — 83 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/Callbacks.cs`
**Purpose:** The two lifecycle callbacks the client posts for itself: "we are connected" and "we are disconnected".
**How it works:** `pub struct ConnectedCallback { base: CallbackMsgBase }` and `pub struct DisconnectedCallback { base: CallbackMsgBase, user_initiated: bool }`. Each implements the four `CallbackMsg` methods by forwarding to the embedded `CallbackMsgBase`; `DisconnectedCallback` adds `user_initiated()`. Both derive `Clone, Copy, Debug, Default`, and `DisconnectedCallback::new(user_initiated)` is the constructor.
**Notes and gotchas:** The C# declares them as *nested* types — `SteamClient.ConnectedCallback` and `SteamClient.DisconnectedCallback` — in a `partial class SteamClient`. `SUMMARY.md` §8 requires the flattening, so in Rust they are top-level types in `steamkit::steam::steam_client`. Their C# constructors are `internal`; the Rust ones are `pub`, because `SteamClient::on_client_connected` / `on_client_disconnected` construct them from another module. `DisconnectedCallback` is `Copy` in Rust (the C# is a reference type), which is why `post_callback_typed` can take it by value and box it. `DisconnectedCallback::user_initiated` is `false` when the disconnection came from a network failure or a forcible server-side disconnect, and `true` when the consumer called `disconnect()` — the same distinction the C# documents. Note the naming collision with the transport-level `DisconnectedEventArgs`, which carries the same flag but lives in `networking::steam3`.

#### `crates/steamkit/src/steam/steam_client/configuration/mod.rs` — 5 lines
**C# counterpart:** none — the C# `Configuration` folder lives in the `SteamKit2` namespace without a namespace of its own; this file is the Rust module list.
**Purpose:** Declares the five submodules of the configuration tree.
**How it works:** `pub mod i_machine_info_provider; pub mod i_steam_configuration_builder; pub mod steam_configuration; pub mod steam_configuration_builder; pub mod steam_configuration_state;`
**Notes and gotchas:** The C# has exactly these five files in `Steam/SteamClient/Configuration/`, so the Rust tree mirrors the C# layout exactly here. `SteamConfigurationState` is an `impl` target on `SteamConfiguration` (the `into_configuration` bridge lives in `steam_configuration.rs`, not here), so the module split does not match the type split.

#### `crates/steamkit/src/steam/steam_client/configuration/i_machine_info_provider.rs` — 17 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/Configuration/IMachineInfoProvider.cs`
**Purpose:** The seam through which the login process obtains the machine-specific identifiers that Steam Guard uses to recognise a machine.
**How it works:** `pub trait IMachineInfoProvider: Send + Sync` with three methods, each returning `Option<Vec<u8>>`: `machine_guid()`, `mac_address()` and `disk_id()`. The default implementation is `crate::util::hardware_utils::MachineInfoProvider::get_default_provider()`, which dispatches to a per-OS provider (`WindowsMachineInfoProvider`, `LinuxMachineInfoProvider`, `MacOSMachineInfoProvider`).
**Notes and gotchas:** The C# returns `byte[]?` from `GetMachineGuid()`, `GetMacAddress()` and `GetDiskId()`; the Rust returns `Option<Vec<u8>>`, which is the same information without the null-versus-empty distinction. The doc comment's warning is important and is inherited verbatim from the C#: **these values must be stable across runs of the same machine, container or VM**, otherwise Steam treats it as a new machine and the user has to re-authenticate. The provider is captured by the configuration at build time and handed to `HardwareUtils::init` by the `SteamClient` constructor, so the machine info is computed in the background and should be ready by logon time. Because the trait is object-safe and the provider is stored as `Arc<dyn IMachineInfoProvider>`, an integrator can substitute its own implementation through `ISteamConfigurationBuilder::with_machine_info_provider` — the documented escape hatch for environments where the default provider cannot find a stable id.

#### `crates/steamkit/src/steam/steam_client/configuration/i_steam_configuration_builder.rs` — 49 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/Configuration/ISteamConfigurationBuilder.cs`
**Purpose:** The interface that configures a `SteamConfiguration` before it is created.
**How it works:** `pub trait ISteamConfigurationBuilder` with eleven `&mut self -> &mut Self` methods: `with_cell_id(u32)`, `with_connection_timeout(Duration)`, `with_default_persona_state_flags(PersonaStateFlags)`, `with_directory_fetch(bool)`, `with_http_client_factory(HttpClientFactory)`, `with_machine_info_provider(Arc<dyn IMachineInfoProvider>)`, `with_protocol_types(ProtocolTypes)`, `with_server_list_provider(Arc<dyn IServerListProvider>)`, `with_universe(EUniverse)`, `with_web_api_base_address(reqwest::Url)`, `with_web_api_key(String)`.
**Notes and gotchas:**

- **The trait is not object-safe.** The `-> &mut Self` return type means `Self` appears in the return position, so the trait cannot be used as `dyn ISteamConfigurationBuilder`. That is harmless — the builder is always the concrete `SteamConfigurationBuilder` — but it means the trait cannot be used for generic plugin-style configuration the way a C# interface could.
- **The C# returns the interface**, so a C# configuration script reads as a chain of `WithX` calls returning `ISteamConfigurationBuilder`. The Rust methods return `&mut Self` (the concrete builder), so the chain still works but the concrete type is what the caller ends up with.
- **Null checks disappear.** The C# throws `ArgumentNullException` for a null server-list provider, base address or API key. The Rust equivalents take non-nullable types (`Arc<dyn IServerListProvider>`, `reqwest::Url`, `String`), so the check is unnecessary — there is no null to pass.
- **The `with_*` methods mutate the builder in place** and return `&mut Self`; nothing is validated beyond the types, exactly as in the C#.
- **The C# names the cell-id setter `WithCellID`** and the API-key setter `WithWebAPIKey`; the Rust port uses snake_case (`with_cell_id`, `with_web_api_key`), matching the crate-wide naming convention.

#### `crates/steamkit/src/steam/steam_client/configuration/steam_configuration_state.rs` — 139 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/Configuration/SteamConfigurationState.cs`
**Purpose:** The raw, mutable values that make up a configuration, plus the explicit bit-mask type that the generated persona-state enum cannot express.
**How it works:**

- `pub struct PersonaStateFlags(i32)` — an explicit bit mask over `EClientPersonaStateFlag`. `DEFAULT` is `PlayerName | Presence | SourceID | GameExtraInfo | LastSeen`. The API: `new(bits)` and `bits()` (both `const fn`), `contains(flag)`, `insert(&mut self, flag)`, `representative()` (the first flag that is set), `union(other)`, a `BitOr` implementation, and `From<EClientPersonaStateFlag> for PersonaStateFlags`.
- `pub struct SteamConfigurationState` with eleven public fields: `allow_directory_fetch: bool`, `cell_id: u32`, `connection_timeout: Duration`, `default_persona_state_flags: PersonaStateFlags`, `http_client_factory: HttpClientFactory`, `machine_info_provider: Arc<dyn IMachineInfoProvider>`, `protocol_types: ProtocolTypes`, `server_list_provider: Arc<dyn IServerListProvider>`, `universe: EUniverse`, `web_api_base_address: reqwest::Url`, `web_api_key: String`.
- `Default for SteamConfigurationState` produces exactly the same values as `SteamConfigurationBuilder::create_default_state()`: directory fetch enabled, cell id 0, a 5-second connection timeout, the default persona flags, the default HTTP client factory, the default machine-info provider, `ProtocolTypes::TCP | ProtocolTypes::WEB_SOCKET`, a `MemoryServerListProvider`, `EUniverse::Public`, `https://api.steampowered.com/` and an empty API key.
- A manual `Debug` implementation hides the trait-object fields behind a placeholder string and prints everything else.

**Notes and gotchas:**

- **The persona-state mask is a fidelity loss that is visible in the public API.** The C# `EClientPersonaStateFlag` is a `[Flags]` enum, so a single field carries the whole mask. The generated Rust enum holds one member per variant and cannot carry flag semantics, so the mask had to be modelled separately. `SteamConfiguration::default_persona_state_flags()` therefore returns only the **first** flag of the mask (`PlayerName`) rather than the whole set, and `SteamConfiguration::default_persona_state_flags_mask()` is the accessor that returns the complete mask. Anything that forwards these flags to Steam must use the mask accessor.
- **`representative()` returns the first matching flag** in the order of `EClientPersonaStateFlag::ALL`, which is why `default_persona_state_flags()` yields `PlayerName` for the default mask.
- **The C#'s state is a plain struct with public fields** and the builder mutates it directly; the Rust port does the same, and `SteamConfigurationState` is `Clone` so that a builder can be reused.
- **The `Default` impl and `create_default_state()` must stay in sync.** They duplicate the same eleven values in two places (`steam_configuration_state.rs` and `steam_configuration_builder.rs`); changing a default in one without the other is a bug.
- **`http_client_factory` and the other trait-object fields are stored as `Arc`s**, so cloning the state clones the references, not the providers.

#### `crates/steamkit/src/steam/steam_client/configuration/steam_configuration_builder.rs` — 112 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/Configuration/SteamConfigurationBuilder.cs`
**Purpose:** Builds a `SteamConfiguration` from a set of options, and defines the defaults the whole library starts from.
**How it works:**

- `pub struct SteamConfigurationBuilder { state: SteamConfigurationState }` with `new()` (a builder holding the default state) and `build(&self) -> SteamConfiguration` (which clones the state and hands it to `SteamConfiguration::from_state`).
- `create_default_state() -> SteamConfigurationState` returns the canonical default state: `allow_directory_fetch: true`, `cell_id: 0`, `connection_timeout: 5 s`, `default_persona_state_flags: PersonaStateFlags::DEFAULT`, `http_client_factory: default_http_client_factory()`, `machine_info_provider: MachineInfoProvider::get_default_provider()`, `protocol_types: ProtocolTypes::TCP | ProtocolTypes::WEB_SOCKET`, `server_list_provider: Arc::new(MemoryServerListProvider::new())`, `universe: EUniverse::Public`, `web_api_base_address: default_web_api_base_address()`, `web_api_key: String::new()`.
- `impl ISteamConfigurationBuilder for SteamConfigurationBuilder` implements the eleven `with_*` methods from `i_steam_configuration_builder.rs`; each mutates `self.state` and returns `self`.

**Notes and gotchas:**

- **The default protocol mask is `TCP | WEB_SOCKET`,** matching the C# `ProtocolTypes = ProtocolTypes.Tcp | ProtocolTypes.WebSocket`. UDP is *not* enabled by default — a server record that offers only UDP will not be used unless the integrator opts in with `with_protocol_types(ProtocolTypes::ALL)`. This matters because `default_connection_factory` prefers **TCP**, then the websocket, then UDP (the C# prefers the websocket first — see the `cm_client.rs` entry for why the order differs), and only the TCP and UDP paths are wrapped in `EnvelopeEncryptedConnection` with the universe's public key.
- **The defaults match the C# exactly** — `AllowDirectoryFetch = true`, `ConnectionTimeout = TimeSpan.FromSeconds(5)`, `MachineInfoProvider = MachineInfoProvider.GetDefaultProvider()`, `ServerListProvider = new MemoryServerListProvider()`, `Universe = EUniverse.Public`, `WebAPIBaseAddress = WebAPI.DefaultBaseAddress`, and an empty API key. `CellID` and `DefaultPersonaStateFlags` are also set the same way.
- **`build()` takes `&self` and clones the state,** so a builder can be reused to produce several configurations. The C#'s `Build()` hands over its state and the configuration then creates its own `SmartCMServerList`; the Rust port produces a fresh `Arc<SmartCMServerList>` per configuration in `into_configuration`, so two configurations built from one builder do not share a server list.
- **The builder is the only mutable step.** Once a `SteamConfiguration` exists it is immutable: the state is behind an `Arc` and every accessor is a getter. The C# documents the same rule ("This object should not be mutated after it is passed to one or more clients").
- **`with_protocol_types` takes the mask as a whole.** There is no per-protocol setter, so enabling UDP means passing `ProtocolTypes::ALL` (or a hand-built mask) — the same as the C#.

#### `crates/steamkit/src/steam/steam_client/configuration/steam_configuration.rs` — 172 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/SteamClient/Configuration/SteamConfiguration.cs`
**Purpose:** The immutable configuration object a client is built from, together with the HTTP-client factory seam and the `HttpClientPurpose` enum.
**How it works:**

- `pub enum HttpClientPurpose { CMWebSocket, WebAPI, CDN }` — the three purposes the C# lists in the same enum (`CMWebSocket`, `WebAPI`, `CDN`).
- `pub type HttpClientFactory = Arc<dyn Fn(HttpClientPurpose) -> reqwest::Client + Send + Sync>` — the counterpart of the C# `delegate HttpClient HttpClientFactory(HttpClientPurpose purpose)`.
- `default_web_api_base_address() -> reqwest::Url` returns `https://api.steampowered.com/`, the counterpart of `SteamKit2.WebAPI.WebAPI.DefaultBaseAddress`.
- `default_http_client_factory() -> HttpClientFactory` returns a closure that builds a `reqwest::Client` with the user agent `SteamKit/4.0.0` (the C# `SteamKit/{Version}` for the pinned 4.0.0-Beta.1).
- `pub struct SteamConfiguration { state: Arc<SteamConfigurationState>, server_list: Arc<SmartCMServerList> }`.
- Constructors: `create(configurator)` takes a `FnOnce(&mut SteamConfigurationBuilder)` and runs it against a fresh builder; `create_default()` builds the default configuration; `from_state(state)` turns a filled-in state into a configuration.
- The accessors: `state()`, `allow_directory_fetch()`, `cell_id()`, `connection_timeout()`, `default_persona_state_flags()`, `default_persona_state_flags_mask()`, `http_client_factory()`, `machine_info_provider()`, `protocol_types()`, `server_list_provider()`, `universe()`, `web_api_base_address()`, `web_api_key()`, `server_list()`.
- `impl SteamConfigurationState { pub fn into_configuration(self) -> SteamConfiguration }` is the bridge: it wraps the state in an `Arc` and creates the matching `Arc<SmartCMServerList>`.

**Notes and gotchas:**

- **The C#'s constructor is `internal`;** the Rust `from_state` is public, so a configuration can be assembled from a hand-built `SteamConfigurationState` without going through the builder.
- **The C#'s `Create` takes an `Action<ISteamConfigurationBuilder>`** — a delegate over the *interface*. The Rust `create` takes a closure over the *concrete* `SteamConfigurationBuilder`, because the trait is not object-safe. A caller therefore gets the concrete builder's methods, which is a superset of the interface's.
- **The HTTP client factory is invoked lazily, not at configuration time.** The C# reaches the Steam Directory through `SteamDirectory.LoadAsync(configuration)` → `configuration.GetAsyncWebAPIInterface("ISteamDirectory")`, which calls `HttpClientFactory(HttpClientPurpose.WebAPI)` at the moment the directory is actually queried. The Rust port preserves this: `steam::discovery::smart_cm_server_list::steam_directory_loader_with_factory` holds the client in a `OnceLock` and calls the factory inside `get_or_init` at the moment of the first fetch, then reuses the client for every later fetch. Building a `SteamConfiguration` therefore never runs the user's factory.
- **`HttpClientPurpose::CMWebSocket` is consumed by the default connection factory.** `default_connection_factory` calls the factory with that purpose and builds a `ReqwestWebSocketFactory`, which performs the upgrade through the configured client; `TungsteniteWebSocketFactory` remains for callers without one.
- **The C#'s factory delegate documents "The HttpClient will be disposed of after use".** The Rust doc comment says the opposite — the returned client is owned by the caller and should be reused, because building one is comparatively expensive. This is the change that makes the lazy invocation above worthwhile.
- **`default_persona_state_flags()` returns only the first flag of the configured mask**, not the whole mask; use `default_persona_state_flags_mask()` for the complete set. See `steam_configuration_state.rs`.
- **One configuration, one server list.** `server_list()` returns the `Arc<SmartCMServerList>` that the configuration created for itself, and every client built on that configuration shares it. The C# behaves the same way (`ServerList = new SmartCMServerList(this)` in the constructor).
- **The configuration is immutable after construction.** The state is behind an `Arc`, the accessors are getters, and the only mutator is the builder.

### 5.7 `crates/steamkit/src/steam/handlers/` — the 15 protocol handlers


Everything in `crates/steamkit/src/steam/handlers/` is one thing: **the translation layer between
the CM connection and the Rust API**. A handler owns a slice of the Steam3 protocol, turns
inbound packets into typed callback structs, and exposes typed methods that send the outbound
half of the same conversation.

| | |
|---|---|
| Handlers | 15 (`steam_apps`, `steam_auth_ticket`, `steam_cloud`, `steam_content`, `steam_friends`, `steam_game_coordinator`, `steam_game_server`, `steam_master_server`, `steam_matchmaking`, `steam_networking`, `steam_screenshots`, `steam_unified_messages`, `steam_user`, `steam_user_stats`, `steam_workshop`) |
| Files | 34 `.rs` (the `mod.rs` of each directory is not counted) |
| C# source | `SteamKit2/SteamKit2/Steam/Handlers/` — 34 `.cs` files, a 1:1 mapping |
| Base class | `ClientMsgHandler.cs` → `client_msg_handler.rs` |

> **Read this section once, top to bottom, before touching any individual handler.** The
> pattern below is identical across all fifteen; the per-file entries that follow only describe
> what is *specific* to each handler.

---

#### The handler pattern

#### The trait

```rust
// crates/steamkit/src/steam/handlers/client_msg_handler.rs
#[async_trait]
pub trait ClientMsgHandler: Send + Sync {
    async fn handle_msg(
        &self,
        client: &SteamClient,
        packet_msg: &dyn IPacketMsg,
    ) -> SteamResult<()>;
}
```

One method. Three design decisions are baked into it:

* **`&self`, not `&mut self`.** A handler is shared. The client stores it as
  `Arc<dyn ClientMsgHandler>` and calls it concurrently; all mutable handler state therefore
  lives behind interior mutability (`DashMap`, `parking_lot::Mutex`, `RwLock`).
* **`client: &SteamClient` is a parameter, not a field.** The C# `ClientMsgHandler` holds a
  strong `SteamClient` reference set by `Setup()`. In Rust that would be a reference cycle
  (client owns handler, handler owns client) and a leak. Every Rust handler therefore stores
  `client: Weak<SteamClient>` for its *own* public methods and receives `&SteamClient` as an
  argument when it needs to reply.
* **`async`.** `handle_msg` may await. No handler in this set currently does, but the contract
  allows it and the C# `HandleMsg` is likewise free to block.

#### How a handler declares which `EMsg` values it owns

**It does not — not in the trait, and not in any registry.** The C# has the same property: the
C# `ClientMsgHandler` base class has no message table either. Ownership is expressed by a
`match` on `packet_msg.msg_type()` **inside** `handle_msg`:

```rust
let callback: Option<Box<dyn CallbackMsg>> = match packet_msg.msg_type() {
    EMsg::ClientLogOnResponse => Some(Box::new(LoggedOnCallback::from_packet(packet_msg)?)),
    EMsg::ClientLoggedOff     => Some(Box::new(LoggedOffCallback::from_packet(packet_msg)?)),
    // …
    _ => None,          // "ignore messages that we don't have a handler function for"
};
if let Some(callback) = callback {
    client.post_callback(callback);
}
Ok(())
```

The consequences are worth stating plainly:

* **Every inbound packet is offered to every handler.** There is no demultiplex table. The
  client walks the handler list and calls each one; a handler that does not recognise the
  `EMsg` returns `Ok(())` and does nothing.
* **There is no compile-time guarantee that a handler owns an `EMsg`.** If two handlers both
  match `EMsg::ClientAccountInfo` — and `steam_user` and `steam_friends` both do — **both** run
  and **both** may post a callback. This is not an accident; see the registration-order note
  below.
* The `_ => None` arm is mandatory. A handler that falls through to `_ => {}` without acting is
  indistinguishable from one that has never heard of the message.

#### Registration

```rust
pub fn add_handler(&self, handler: Arc<dyn ClientMsgHandler>);            // untyped
pub fn add_handler<T: ClientMsgHandler + 'static>(&self, handler: T) -> Result<(), SteamError>;
pub fn remove_handler(&self, handler: &Arc<dyn ClientMsgHandler>) -> bool;
pub fn remove_handler_type<T: ClientMsgHandler + 'static>(&self) -> bool;
pub fn get_handler<T: ClientMsgHandler + 'static>(&self) -> Option<Arc<T>>;
pub fn get_required_handler<T: ClientMsgHandler + 'static>(&self) -> Result<Arc<T>, SteamError>;
```

`add_handler` records the concrete type, so the handler stays retrievable with `get_handler`. A second
handler of the same `TypeId` yields `SteamError::InvalidArgument`, matching the C# `AddHandler`.
`get_required_handler` is the C# `GetRequiredHandler<T>()`; the C# threw `InvalidOperationException`,
the Rust port returns `SteamError::NotFound`.

`SteamClient::add_default_handlers` installs all fifteen, in this order:

```
SteamFriends → SteamUser → SteamApps → SteamGameCoordinator → SteamGameServer
→ SteamUserStats → SteamMasterServer → SteamCloud → SteamWorkshop → SteamUnifiedMessages
→ SteamScreenshots → SteamMatchmaking → SteamNetworking → SteamContent → SteamAuthTicket
```

**`SteamFriends` must precede `SteamUser`.** Both match `EMsg::ClientAccountInfo`. `SteamFriends`
uses it to cache the local persona name; `SteamUser` uses it to post `AccountInfoCallback`.
Because handlers run in registration order, a subscriber of `AccountInfoCallback` therefore sees
a `SteamFriends` whose `get_persona_name()` is already populated. Swap the two and the
`AccountInfoCallback` subscriber would observe an empty persona name. This is the single place
where handler registration order is load-bearing.

#### The dispatch loop

`SteamClient::on_client_msg_received(&self, packet)` — private, called by `CMClient` when a
packet has been decoded and framed. It is also reachable from tests through
`SteamClient::receive_test_packet_msg(packet)`, which feeds a packet into the pipeline without
a live connection.

1. **Job housekeeping first**, before any handler sees the packet:
   * `EMsg::JobHeartbeat` → `job_manager.heartbeat_job(packet.target_job_id())` — extends that
     job's deadline by `JOB_HEARTBEAT_EXTENSION` (10 s).
   * `EMsg::DestJobFailed` → `job_manager.fail_job(packet.target_job_id())`.
2. **Snapshot the handler list.** `Vec<Arc<dyn ClientMsgHandler>>` is cloned out of the
   `RwLock`, so a handler may add or remove handlers while it runs without deadlocking.
3. **Call each handler in turn**, `handler.handle_msg(self, packet).await`.
4. **An `Err` from any handler is fatal to the connection.** The client logs
   `"Unhandled exception from handler: {error}"` and calls `cm.disconnect_with(false).await`,
   returning `false`. The C# behaves the same way: an exception escaping `HandleMsg` tears down
   the CM connection. A malformed packet therefore takes the whole client down, not just the
   handler.
5. Returns `true` when every handler accepted the packet.

#### The two-part file convention

```
handlers/
├── client_msg_handler.rs               ← the trait
├── mod.rs                              ← 15 `pub mod` lines, nothing else
└── steam_user/
    ├── mod.rs                          ← `pub mod callbacks; pub mod steam_user;`
    ├── steam_user.rs                   ← the handler + its public API
    └── callbacks.rs                    ← the callback structs it emits
```

* **`steam_<name>.rs`** holds the handler struct `SteamXxx`, the public methods that send
  requests, the `impl ClientMsgHandler for SteamXxx`, and the *request-side* types: the
  `*Details` structs (`LogOnDetails`, `QueryDetails`, `StatusDetails`, `ScreenshotDetails`,
  `EnumerationUserDetails`), request helper types (`PICSRequest`), and module-level free
  functions that encode or decode wire payloads.
* **`callbacks.rs`** holds every `CallbackMsg` the handler emits, plus the *value* types those
  callbacks carry (`License`, `PICSProductInfo`, `PICSChangeData`, `ChatMemberInfo`,
  `LeaderboardEntry`, `File`, `Server`, …).

The C# nests the callback classes inside the handler class (`SteamUser.LoggedOnCallback`). Rust
has no nested types, so they live in a sibling module and are addressed as
`steam::handlers::steam_user::callbacks::LoggedOnCallback`. The `mod.rs` files contain **only**
`pub mod` declarations — there are no re-exports, so the path above is the real path.

The convention has exactly three exceptions, and each mirrors a real C# file:

| Extra file | Why |
|---|---|
| `steam_auth_ticket/ticket_info.rs` | the C# has `SteamAuthTicket/TicketInfo.cs` — a disposable holder of the generated ticket bytes |
| `steam_matchmaking/lobby.rs`, `lobby_cache.rs` | the C# has `SteamMatchmaking/Lobby.cs` and `LobbyCache.cs` |
| `steam_friends/friend_cache.rs` | the C# has `SteamFriends/FriendCache.cs` |

And exactly one directory without a `callbacks.rs`: **`steam_content`**. `SteamContent` speaks
only unified messages and returns plain structs from its `async fn`s — it never posts a
`CallbackMsg`.

#### How a callback is subscribed to and dispatched

Every callback implements one trait:

```rust
pub trait CallbackMsg: Any + Send + Sync {
    fn job_id(&self) -> JobID;
    fn set_job_id(&mut self, id: JobID);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

`CallbackMsgBase` is the reusable job-id storage that callback structs embed (or they hold a
plain `job_id: JobID` field and forward the four calls by hand — both patterns appear in this
tree; the `steam_apps` and `steam_matchmaking` callbacks use `CallbackMsgBase`, the
`steam_user` and `steam_friends` ones use the raw field).

`CallbackManager` (`crates/steamkit/src/steam/steam_client/callback_mgr/`) holds
`subscriptions: RwLock<Vec<(SubscriptionId, Arc<CallbackSubscription>)>>` and offers:

| Method | Restriction |
|---|---|
| `subscribe::<T>(Box<dyn Fn(&T) + Send + Sync>) -> SubscriptionId` | every callback of type `T` |
| `subscribe_job::<T>(job_id, Box<dyn Fn(&T) + Send + Sync>)` | one job only |
| `subscribe_async::<T>(Box<dyn Fn(Arc<T>) -> BoxFuture<'static, ()> + Send + Sync>)` | every callback of type `T`, awaited by the caller |
| `subscribe_job_async::<T>(job_id, …)` | one job only, awaited |
| `subscribe_guarded::<T>(f) -> SubscriptionGuard<'_>` | every callback of type `T`; unsubscribes when the guard drops |
| `subscribe_service_notification[_async]`, `subscribe_service_response[_async]` | same machinery, named for the unified-messages use case; they create the service and register the decoder first, like the C# `SubscribeServiceNotification` |

**Dropping the `SubscriptionId` does not unsubscribe; dropping a `SubscriptionGuard` does.** The C#
`Subscribe` methods return an `IDisposable` that unsubscribes on `Dispose`, not on losing the
reference; here `unsubscribe(id)` is that `Dispose`. `SubscriptionId` is a `Copy` token, while
`subscribe_guarded` returns a guard whose `Drop` calls it, the `using` form.

Dispatching is `CallbackManager::dispatch(msg)` → `dispatch_arc(Arc<dyn CallbackMsg>)`:

1. Compute the message's `job_id`.
2. For every subscription, test `subscription.matches(msg, job_id)`:
   * `CallbackMsg::is_a` decides the type match. The default is an **exact** `TypeId` match;
     a callback that models a hierarchy overrides `as_a` to expose its base view, so a base
     subscription receives it, like the C# `CallbackType.IsAssignableFrom(type)`.
   * `job_id`: `None` matches everything; `Some(id)` matches only that job — **except**
     `JobID::INVALID`, which matches everything.
3. Sync handlers are invoked inline with `&T`. Async handlers are spawned onto the current
   Tokio runtime via `tokio::runtime::Handle::try_current()` — and **silently dropped when no
   runtime is running**, because there is nothing to drive them. `dispatch_arc_async` awaits
   them instead of spawning.

The type erasure is undone inside the subscription: `Callback<TCall>::into_subscription` wraps
the typed closure into a `Box<dyn Fn(&dyn CallbackMsg)>` that does
`msg.as_any().downcast_ref::<TCall>()`. The `TypeId` already matched, so the downcast cannot
fail; the `if let` is belt and braces.

#### From a packet to a typed callback

```text
CM socket
  │  CMClient decodes the frame into a PacketMsg
  ▼
SteamClient::on_client_msg_received(packet)
  │  job housekeeping (heartbeat / dest-job-failed)
  │  for each handler: handler.handle_msg(self, packet).await
  ▼
SteamUser::handle_msg
  │  match packet.msg_type() { EMsg::ClientLogOnResponse => … }
  │  LoggedOnCallback::from_packet(packet)   ← decodes the protobuf
  ▼
client.post_callback(Box::new(callback))
  │  job_manager.try_complete_job(job_id, &*msg)   ← the waiting future gets its clone
  │  callback_queue_tx.send(msg)                   ← the queue keeps the original
  ▼
tokio::sync::mpsc channel (unbounded)
  │
  ├─► SteamClient::get_callback() / wait_for_callback() …   (the raw queue)
  └─► SteamClient::run_callbacks() / run_wait_callbacks() → CallbackManager::dispatch(…)
        └─► every matching subscription's closure, with the concrete &T
```

Two details in `post_callback` matter:

* **A callback goes to two owners at once** — the job that is waiting for it, and the callback
  queue. C# can share one reference between them; Rust cannot hand the same
  `Box<dyn CallbackMsg>` to both, so the job manager **clones the concrete callback** for the
  job (`clone_callback_of::<T>`). This is why every callback struct in this tree derives
  `Clone`.
* The job is completed **before** the callback enters the queue, so a handler awaiting a job
  and a subscriber to the same callback type are woken in that order.

#### The `JobID` correlation

This is the mechanism that makes `async fn` request methods possible, and it is the C#
mechanism with the names changed.

**C#.** A request method calls `client.GetNextJobID()` to allocate an id, calls
`client.StartJob(jobId)` to obtain an `AsyncJob<T>`, writes the id into the outgoing message's
**source job id** field, and returns the job to the caller. Steam echoes the id back in the
**target job id** of the response. `AsyncJobManager.TryCompleteJob` looks the in-flight job up
by that id, type-checks the callback against the job's expected type, and completes the
`TaskCompletionSource`. `EMsg.JobHeartbeat` extends the job's deadline; `EMsg.DestJobFailed`
fails it.

**Rust.** The same shape:

| Step | C# | Rust |
|---|---|---|
| allocate | `client.GetNextJobID()` | `client.get_next_job_id()` → `(process_start_time << 20) \| (sequence & 0xF_FFFF)`, `sequence` from an `AtomicU64`, `process_start_time` captured once at client construction |
| register | `client.StartJob(jobId)` | `client.start_job::<T>(job_id) -> AsyncJob<T>` — inserts a `JobEntry` into the manager's `DashMap<JobID, JobEntry>` |
| stamp | `msg.SourceJobID = jobId` | `msg.set_source_job_id(job_id)` |
| await | `await job.Task` | `job.wait().await` |
| complete | `AsyncJobManager.TryCompleteJob` | `AsyncJobManager::try_complete_job(job_id, callback)` |

`try_complete_job` has three branches:

* **No job with that id** → the callback is ignored. Same as C#.
* **A single-result job** (no finish condition): if the callback's `TypeId` is not the one the
  job was started for, the job is failed with
  `SteamError::Malformed("callback type … does not match the expected type of job …")`.
  Otherwise the callback is cloned and pushed through the job's `oneshot`.
* **A multi-result job** (started with `start_job_multiple`): the callback is appended to the
  result set; if the finish condition is satisfied the job completes with the accumulated
  `ResultSet<T>`, otherwise the job is **heartbeated** — its deadline is extended — and waits
  for more. This is how `PICSGetProductInfo` accumulates several responses into one result.

Timeouts and cancellation:

* `DEFAULT_JOB_TIMEOUT = 10 s`, swept by a `tokio::time::interval` task every
  `JOB_TIMEOUT_CHECK_INTERVAL = 1 s` while the client is connected. A timed-out job fails with
  `SteamError::Timeout(duration)`.
* `EMsg::JobHeartbeat` extends the deadline by `JOB_HEARTBEAT_EXTENSION = 10 s`.
* On disconnect, `cancel_pending_jobs()` fails every job with `SteamError::Cancelled` — except a
  multi-result job, which completes with whatever it has accumulated (`complete = false`,
  `failed = false`).
* `SteamClient::start_job` fails the job immediately with `SteamError::JobFailed` when the
  client is not connected.

**Naming.** The C# defines three classes in `Types/JobID.cs`: `AsyncJob` (abstract base),
`AsyncJob<T>` and `AsyncJobMultiple<T>`. Rust forbids a non-generic and a generic type sharing
a name in one module, so the port has **one** type, `AsyncJob<T = AnyCallback>`, with
`AnyCallback` standing in for the untyped base. The canonical names live in
`crates/steamkit/src/types/job_id.rs`:

| C# | Rust canonical | Role alias |
|---|---|---|
| `AsyncJob` (base) | `AsyncJob<AnyCallback>` | `JobState` |
| `AsyncJob<T>` | `AsyncJob<T>` | `JobHandle<T>` |
| `AsyncJobMultiple<T>` | `AsyncJobMultiple<T>` | `JobGroup<T>` |
| `ResultSet` | `ResultSet<T>` | `JobResultSet<T>` |
| `Task<ResultSet>` | `MultiJobReceiver<T>` | `MultiJob<T>` |
| `Task<T>` | `TypedJobReceiver<T>` | `TypedJob<T>` |

The three role aliases — `JobState`, `JobHandle<T>`, `JobGroup<T>` — exist so the three jobs can
be named without the `AsyncJob`/`AsyncJob<T>` collision; the canonical names remain the real
types. **Where they matter in this section:** `client.start_job::<T>(job_id)` returns the
`AsyncJob<T>` (aka `JobHandle<T>`) that a handler awaits, and
`client.job_manager().start_job_multiple::<T>(job_id, finish_condition)` returns the
`AsyncJobMultiple<T>` (aka `JobGroup<T>`) whose await yields a `ResultSet<T>` — used by
`SteamApps::pics_get_product_info_for_ids`.

#### The C# class versus the Rust trait + struct pair

| C# | Rust |
|---|---|
| `public abstract class ClientMsgHandler` | `pub trait ClientMsgHandler: Send + Sync` |
| `protected SteamClient Client { get; private set; }`, set by `internal void Setup(SteamClient)` | `client: Weak<SteamClient>` field on the handler struct; `&SteamClient` handed to `handle_msg` as an argument |
| `protected bool ExpectDisconnection { get; set; }` proxying to `Client.ExpectDisconnection` | `SteamClient::expect_disconnection()` / `set_expect_disconnection(bool)`; the trait deliberately does **not** carry the flag |
| `public abstract void HandleMsg(IPacketMsg)` | `async fn handle_msg(&self, client: &SteamClient, packet_msg: &dyn IPacketMsg) -> SteamResult<()>` |
| `public sealed partial class SteamUser : ClientMsgHandler`, callback classes nested inside | `pub struct SteamUser { client: Weak<SteamClient> }` + `impl ClientMsgHandler for SteamUser` + a sibling `callbacks` module |
| `Client.GetHandler<SteamUser>()` | `client.get_handler::<SteamUser>() -> Option<Arc<SteamUser>>` |
| a handler that throws | a handler that returns `Err` — which **disconnects the client** |
| `Task<TicketInfo>` | `async fn -> Result<TicketInfo, SteamError>` |

Three consequences of the weak back-reference that show up in every handler below:

1. **No reference cycle.** The client owns `Arc<dyn ClientMsgHandler>`; the handler owns
   `Weak<SteamClient>`. A strong `Arc` inside the handler would keep the client graph alive
   forever.
2. **Every public method starts with `let client = self.client()?;`**, which fails with
   `SteamError::Connection("SteamClient has been dropped")` once the client is gone.
3. **`expect_disconnection` is a client flag, not a handler flag.** `SteamUser::log_off` and
   `SteamGameServer::log_off` set it through the client, and the resulting `DisconnectedCallback`
   carries `UserInitiated = true`.

Two more port-wide conventions:

* **Public callback accessors, private constructors.** Every callback has a
  `pub(crate) fn from_packet(&dyn IPacketMsg) -> SteamResult<Self>` (or `from_body` /
  `from_response` / `from_parts` / `new`) and `pub fn` getters. The C# exposed public fields;
  the Rust port uses accessors so that the `Debug` and the wire format can evolve separately.
* **Manual `Debug` impls.** Several handlers print only counts (`SteamMatchmaking` prints the
  number of pending manipulations, `SteamAuthTicket` the token and ack counts) so that a
  `tracing` statement cannot leak a ticket or a token.

#### `crates/steamkit/src/steam/handlers/client_msg_handler.rs` — 26 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/ClientMsgHandler.cs`
**Purpose:** Declares the base requirements every message handler implements. The C# file
declares `public abstract class ClientMsgHandler`, the base of the fifteen handlers; every
derived handler overrides `HandleMsg(IPacketMsg)` and uses the protected `Client` property to
send replies. The Rust port turns the class into the trait.
**How it works:** The whole file is the 51-line trait shown above plus the module documentation.
`handle_msg` takes `&self` (handlers are shared, so their mutable state is behind interior
mutability), takes the `&SteamClient` as an argument (so a handler can reply without holding a
strong reference to the client, which would create a reference cycle), and returns
`SteamResult<()>` — an `Err` disconnects the client. The trait is `Send + Sync` because the
client stores it as `Arc<dyn ClientMsgHandler>` and may call it from any task.
**Notes and gotchas:** The C# `Setup(SteamClient)` + `Client` property becomes a
`Weak<SteamClient>` back-reference stored on every handler struct. The C#
`ClientMsgHandler.ExpectDisconnection` property proxies to `CMClient.ExpectDisconnection`; in
Rust the flag lives on the `SteamClient` itself and is reached through
`SteamClient::set_expect_disconnection`, so the trait deliberately does **not** carry it. The
trait shape is frozen by the porting contract §12.

---

#### `crates/steamkit/src/steam/handlers/steam_apps/callbacks.rs` — 1476 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamApps/Callbacks.cs`
**Purpose:** The callback family of the apps/packages handler. It covers everything Steam
reports about a user's licences and about PICS (the package/app metadata service): the
licence list, free licences, ownership and depot tickets, legacy game keys, game connect
tokens, VAC status, the three PICS responses, guest passes, purchases, beta passwords and
private betas.
**How it works:** Every type follows the same shape — a `#[derive(Clone, Debug)]` struct, a
`pub(crate) fn from_packet(&dyn IPacketMsg) -> SteamResult<Self>` constructor, a set of `pub
fn` accessors, and an `impl CallbackMsg` block. The five that matter most:

* **`LicenseListCallback`** — fired during logon and again whenever the licence set changes
  (purchase, key redemption, refund). Carries an `EResult` and a `Vec<License>`. `License` is
  the richest value type in the tree: package id, change number, creation and next-process
  times, minute limit and minutes used, payment method, licence flags, purchase country,
  licence type, territory code, owner account id, the PICS access token for the package and
  the master package id.
* **`PICSProductInfoCallback`** — the multi-response one. Carries `meta_data_only`,
  `response_pending`, the unknown app/package id lists, and `BTreeMap<u32, PICSProductInfo>`
  for the apps and the packages. `PICSProductInfo` holds the id, change number,
  `missing_token`, the SHA hash, the decoded `KeyValue` tree, `only_public`, and the HTTP
  fallback: `use_http` is true when there is a non-empty SHA *and* a non-empty `http_host`
  *and* the buffer is at least `http_min_size` bytes; `http_uri()` then builds
  `http://{host}/appinfo/{id}/sha/{hex}.txt.gz`.
* **`PICSChangesCallback`** — the change-number bookkeeping: `last_change_number`,
  `current_change_number`, the three `requires_full_*_update` flags and
  `BTreeMap<u32, PICSChangeData>` for the app and package changes. `PICSChangeData` is the
  per-id triple `(id, change_number, needs_token)`.
* **`PICSTokensCallback`** — the PICS access tokens, as `BTreeMap<u32, u64>` for apps and
  packages, plus the denied id lists. These tokens are what `PICSRequest` carries.
* **`AppOwnershipTicketCallback`** — `result`, `app_id` and the raw ticket bytes. It is the
  input to `SteamAuthTicket`'s ticket generation, which is why it is worth reading.

The rest follow the same pattern: `FreeLicenseCallback`, `DepotKeyCallback`,
`LegacyGameKeyCallback`, `GameConnectTokensCallback`, `VACStatusCallback`,
`GuestPassListCallback`, `RedeemGuestPassResponseCallback`, `PurchaseResponseCallback`,
`CheckAppBetaPasswordCallback`, `PrivateBetaCallback`.

**Notes and gotchas:** The C# `Dictionary<uint, ulong>` and `Dictionary<uint, PICSProductInfo>`
become `BTreeMap`s so that iteration order — and with it the order of any encoded output — is
deterministic. `ELicenseFlags` and `EPaymentMethod` are `[Flags]` enums in C# and can hold a
*combination* of bits; the generated Rust enum cannot, so `License` keeps **both** the enum
value and the raw bit pattern (`license_flags()`, `license_flags_value()`,
`payment_method()`, `payment_method_value()`). `ERESULT_DEFAULT = 2` (`EResult::Fail`) is the
value assumed when an `eresult` field is absent from the wire. The `PayloadReader` adapter
over `MsgBase` is the subtlest thing in the file: it hands out **one byte per `read` call**,
because the C# hands the `MsgBase` stream to a `BinaryReader`/`KeyValue.TryReadAsBinary` that
consumes it as it goes, while the Rust binary `KeyValue` reader buffers 4 KiB at a time and
would move the payload position far past the entry actually parsed, corrupting every entry
after the first.

---

#### `crates/steamkit/src/steam/handlers/steam_apps/steam_apps.rs` — 460 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamApps/SteamApps.cs`
**Purpose:** Talks to the app and package side of the Steam network: ownership tickets, depot
decryption keys, the three PICS exchanges, free licences, beta passwords, private betas and
legacy game keys.
**How it works:** `pub struct SteamApps { client: Weak<SteamClient> }` with
`pub fn new(client: &Arc<SteamClient>) -> Self`. The public methods are all `async fn` and all
follow the same three-step shape: allocate a job id, `client.start_job::<Callback>(job_id)`,
send the message, `job.wait().await`. The methods are `get_app_ownership_ticket`,
`get_depot_decryption_key`, `pics_get_access_tokens` / `pics_get_access_tokens_for_ids`,
`pics_get_changes_since`, `pics_get_product_info` / `pics_get_product_info_for_ids`,
`request_free_license` / `request_free_licenses`, `check_app_beta_password`,
`get_legacy_game_key` and `pics_get_private_beta`. The request-side type `PICSRequest` carries
an app or package id together with the PICS access token that `PICSTokensCallback` handed out.
`handle_msg` matches fifteen `EMsg` values and posts the matching callback for each.
**Notes and gotchas:** The C# methods return an `AsyncJob<T>`; the Rust port follows
the porting contract §5.1 and returns `async fn -> Result<T, SteamError>` — the caller
awaits the callback directly instead of holding a job. `PICSGetProductInfo` is the one
multi-result request: the C# returns an `AsyncJobMultiple<PICSProductInfoCallback>`, the Rust
port starts a multi job with the finish condition `!callback.response_pending()` and returns
its `ResultSet<PICSProductInfoCallback>`. Because it is a multi job, the manager heartbeats it
on every partial response rather than timing it out — which is how Steam's chunked PICS
responses are collected into one result.

---

#### `crates/steamkit/src/steam/handlers/steam_auth_ticket/callbacks.rs` — 150 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamAuthTicket/Callbacks.cs`
**Purpose:** The two callbacks of the auth-ticket handler: the acknowledgement of the auth
list and the completion of a ticket authentication.
**How it works:** `TicketAcceptedCallback` is built from a `ClientAuthListAck` and carries the
app ids that have generated tickets, the CRC32 hashes of the activated tickets, and the message
sequence number. `TicketAuthCompleteCallback` is built from a `ClientTicketAuthComplete` and
carries the `EAuthSessionResponse`, the authentication state, the game id, the owner SteamID,
the game-server SteamID, the ticket CRC and the ticket sequence.
**Notes and gotchas:** `TicketAuthCompleteCallback::auth_session_response` decodes to a typed
`EAuthSessionResponse` and falls back to `EAuthSessionResponse::UserNotConnectedToSteam` when
the raw value is not a member of the enumeration. The `SteamGameServer` handler receives the
**same** `EMsg::ClientTicketAuthComplete` message and posts its own
`TicketAuthCallback` — two handlers, one message, two callbacks, both fired.

---

#### `crates/steamkit/src/steam/handlers/steam_auth_ticket/steam_auth_ticket.rs` — 423 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamAuthTicket/SteamAuthTicket.cs`
**Purpose:** Generates authentication session tickets — the blobs a game server uses to prove
that a client owns the game — and tracks Steam's acknowledgement of them.
**How it works:** `pub struct SteamAuthTicket` holds four pieces of state: a
`Mutex<VecDeque<Vec<u8>>>` of game connect tokens, a `Mutex<HashMap<u32, Vec<CMsgAuthTicket>>>`
of the outstanding tickets per app id, a `DashMap<u32, Arc<Notify>>` of per-CRC acknowledgement
waiters, and the weak client reference. `get_auth_session_ticket(appid)` and
`get_auth_ticket_for_web_api(appid, identity)` both funnel into
`get_auth_session_ticket_internal`, which:

1. requires `client.cell_id()` to be present (otherwise `SteamError::Connection("User not
   logged in.")`);
2. asks the `SteamApps` handler for the app ownership ticket and fails if its result is not
   `EResult::OK`;
3. pops one game connect token from the deque, failing with `"There's no available game
   connect tokens left."` when the queue is empty;
4. builds the ticket with `build_auth_ticket` — the token length, the token bytes, a session
   size, the ticket type (`AuthSession = 2` or `WebApiTicket = 5`), 8 random bytes, a monotonic
   timestamp and a process-global sequence number;
5. computes the CRC32 of the ticket, registers it in `tickets_by_game`, registers the
   acknowledgement waiter **before** sending, and sends the whole auth list;
6. waits up to `TICKET_ACK_TIMEOUT = 10 s` for Steam to acknowledge that specific CRC;
7. combines the auth ticket and the ownership ticket into the final token.

`send_tickets` flattens the per-app ticket map into the `ClientAuthList` message (with
`tokens_left` = the number of connect tokens still held) and sends it with a fresh source job
id. `cancel_auth_ticket` removes the ticket from the app's list and re-sends the auth list, so
that Steam stops honouring it — this is what `TicketInfo`'s `Drop` calls.
`handle_msg` handles `ClientAuthListAck`, `ClientTicketAuthComplete`, `ClientGameConnectTokens`
and `ClientLogOff` (which clears the connect tokens).
**Notes and gotchas:** The acknowledgement is correlated on the **ticket CRC**, not on the job
id. Steam stopped reliably re-acknowledging repeated auth lists after its 2025-08 change
(issue #1567), so the C# original was changed the same way; a missing acknowledgement is
logged as a warning and the ticket is returned anyway, because it validates server-side through
`BeginAuthSession`. The waiters are `tokio::sync::Notify` rather than
`TaskCompletionSource<bool>`. `combine_tickets` pads a Web API ticket to exactly 2560 bytes
(`WEB_API_TICKET_SIZE`) and fills the padding with random bytes, because the C# leaves
uninitialised heap memory there. `CheckAppBetaPasswordCallback::from_packet` decodes each beta password from hex and returns an error on malformed input, which the client turns into a disconnect like the C# `Convert.FromHexString` exception.

---

#### `crates/steamkit/src/steam/handlers/steam_auth_ticket/ticket_info.rs` — 72 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamAuthTicket/TicketInfo.cs`
**Purpose:** Holds a generated, valid session ticket and ties its lifetime to the ticket's
validity: when the `TicketInfo` is dropped, the ticket is withdrawn from Steam's auth list.
**How it works:** `pub struct TicketInfo { client: Weak<SteamClient>, app_id: u32, ticket:
Vec<u8>, ticket_crc: u32 }`. `TicketInfo::new` computes the CRC32 of the ticket bytes.
`ticket()` hands out the bytes; `app_id()` and `ticket_crc()` are crate-private because only
the handler needs them.
**Notes and gotchas:** The `Drop` implementation does two things: it looks the handler up
through the weak client reference and calls `cancel_auth_ticket(self)`, and then it **wipes the
ticket bytes** (`self.ticket.fill(0)`) so the secret does not linger in memory. The handler is
looked up through the client rather than held directly, because holding it would keep the
whole client alive for as long as the ticket exists — the same reference-cycle discipline as
everywhere else in this tree. The C# `TicketInfo` is `IDisposable`; the Rust port expresses the
same intent with `Drop`.

#### `crates/steamkit/src/steam/handlers/steam_cloud/callbacks.rs` — 201 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamCloud/Callbacks.cs`
**Purpose:** The three result types of the Steam Cloud service — the details of a piece of user
generated content, the info of a single file in the user's cloud storage, and the result of
publishing (sharing) a cloud file.
**How it works:** `UGCDetailsCallback` carries the result, app id, UGC id, the creator's
SteamID, the content URL, the file name, the file size, the timestamp, the file SHA, the
compressed size and the range-check host. `SingleFileInfoCallback` carries the result, app id,
file name, the SHA hash bytes, the timestamp, the raw file size and the `is_explicit_delete`
flag. `ShareFileCallback` carries the result and the resulting UGC handle.
**Notes and gotchas:** **These three types are not `CallbackMsg` implementations.** The module
header says so explicitly: in C# they are plain result classes returned from the `Task` of the
request, and the Rust port keeps them that way — they are built with `from_response(result,
response)` rather than `from_packet`, they carry no job id, and they are never posted to the
callback manager. The name "Callback" is inherited from the C# and is misleading here: they are
results, not notifications.

---

#### `crates/steamkit/src/steam/handlers/steam_cloud/steam_cloud.rs` — 157 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamCloud/SteamCloud.cs`
**Purpose:** Talks to the `Cloud` service of the Steam unified-messages service: fetching the
details of a piece of UGC, fetching the info of a single file in the user's cloud storage, and
committing a cloud file so that its UGC handle becomes publicly visible.
**How it works:** `pub struct SteamCloud { client: Weak<SteamClient>, service:
OnceLock<Arc<UnifiedService>> }`. The `service()` accessor lazily asks the
`SteamUnifiedMessages` handler for a service named `"Cloud"` (`SERVICE_NAME`), declares the
response types for `GetFileDetails`, `GetSingleFileInfo` and `ShareFile` with
`set_response_type::<T>(method)`, and memoises the handle in the `OnceLock`. The three public
methods — `request_ugc_details`, `get_single_file_info` and `share_file` — each build the
request protobuf, call
request protobuf, call `service.send_message::<Request, Response>(method, request)?`, await the
returned `AsyncJob` with `.wait().await`, and convert the `ServiceMethodResponse` into the
corresponding result type.
its own. Everything goes through the unified-messages service, and the response arrives as a
`ServiceMethodResponse` on the job the service call registered. The C# asks
`Client.GetHandler<SteamUnifiedMessages>()!.CreateService<Cloud>()`; the Rust port asks for a
service **by name**, because `prost` does not generate the C#'s typed service classes.

---

#### `crates/steamkit/src/steam/handlers/steam_content/steam_content.rs` — 239 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamContent/SteamContent.cs`
**Purpose:** Talks to the `ContentServerDirectory` service of the Steam unified-messages
service: listing the content servers for a Steam pipe, obtaining a manifest request code, and
obtaining a CDN authentication token.
**How it works:** Same shape as `SteamCloud`: a `OnceLock<Arc<UnifiedService>>` memoising a
service named `"ContentServerDirectory"` whose response types are declared for
`GetServersForSteamPipe`, `GetManifestRequestCode` and `GetCDNAuthToken`. The public methods
are `get_servers_for_steam_pipe(cell_id, max_num_servers)`,
`get_manifest_request_code(depot_id, app_id, manifest_id, branch, branch_password_hash)` and
`get_cdn_auth_token(app, depot, host_name)`. `get_servers_for_steam_pipe` converts the
response with `ContentServerDirectoryService::convert_server_list`, reusing the web-API
implementation's conversion so both paths produce the same `Server` values.
**Notes and gotchas:** **This is the only handler with no `callbacks.rs`** — it emits no
`CallbackMsg` at all, and `handle_msg` is empty. Its results are plain values: a
`Vec<Server>`, a `u64` manifest request code, and a `CDNAuthToken` (the C#
`SteamContent.CDNAuthToken`, a struct with `result`, `token` and `expiration`). The branch
handling is worth reading: the `"public"` branch is the default branch and never carries a
password, so a request for `"public"` is sent with **no** branch and **no** password hash;
and supplying a password hash without a branch name is rejected with
`SteamError::InvalidArgument("Branch name may not be null if password is provided.")`. Note
that the C#'s `GetServersForSteamPipe` is the alternative to the web API's
`ContentServerDirectoryService.Load` — the same data over the CM connection instead of over
HTTPS.

#### `crates/steamkit/src/steam/handlers/steam_friends/callbacks.rs` — 1738 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamFriends/Callbacks.cs`
**Purpose:** The callback family of the friends handler. It covers three overlapping
conversations: the **friend list** (who is my friend, what is their relationship, what is their
persona state), **person-to-person chat** (direct messages, their history, the friend request
reply), and **chat rooms** (entering, messages, member changes, room info, actions, invites).
It also carries the clan state and the profile info of a friend.
**How it works:** Every type follows the house pattern — `#[derive(Clone, Debug)]` struct,
`pub(crate)` constructor, `pub fn` accessors, `impl CallbackMsg`. The five that matter most:

* **`PersonaStateCallback`** — the most-read callback in the library. Fired once per friend
  entry in a `ClientPersonaState` message. It carries the `status_flags` (which say *what
  changed*: player name, presence, game data blob), the friend's SteamID, the persona state and
  state flags, the game app id / game id / game name, the game server IP and ports, the source
  SteamID, the raw game data blob, the player name, the avatar hash, the last log-off and
  log-on times, the clan rank and tag, and the number of online session instances.
* **`FriendsListCallback`** — the friend list itself: an `incremental` flag (whether this
  message is a delta or the complete list) and a `Vec<Friend>`, where `Friend` is a
  `(steam_id, relationship)` pair.
* **`ChatEnterCallback`** — the result of joining a chat room: the chat id, the friend id, the
  room type, the owner id, the clan id, the chat flags, the enter response, the member count,
  the room name and the member list (`Option<Vec<ChatMemberInfo>>`).
* **`FriendMsgHistoryCallback`** — the result of a history request: the result, the SteamID of
  the other party and a `Vec<FriendMessage>`, each with the sender SteamID, an `unread` flag,
  the text and the timestamp. Offline messages are exactly the ones with `unread == true`.
* **`ProfileInfoCallback`** — the full profile of a friend: result, SteamID, account creation
  time, real name, city, state, country, headline and summary.

The rest: `ClanStateCallback` (with `ClanEvent` for the clan's events and announcements),
`FriendMsgCallback` (incoming direct message), `FriendMsgEchoCallback` (the echo of a message
sent from another instance), `FriendAddedCallback`, `ChatMsgCallback`,
`ChatMemberInfoCallback` (with `StateChangeDetails`), `ChatRoomInfoCallback`,
`ChatActionResultCallback`, `ChatInviteCallback`, `IgnoreFriendCallback` and
`PersonaChangeCallback`.
**Notes and gotchas:** Three unusual things are worth knowing before reading this file.

* **`key_value_as_enum` reimplements `KeyValue.AsEnum<T>`.** The C# uses `Enum.TryParse`,
  which accepts *either* the member name *or* the numeric value of the member. The Rust
  helper therefore tries the numeric parse first and falls back to the name, against the
  explicit member tables `CLAN_PERMISSION_MEMBERS` and `CHAT_PERMISSION_MEMBERS`. It is used
  by `ChatMemberInfo::details()` (`EClanPermission`) and `ChatMemberInfo::permissions()`
  (`EChatPermission`), which read their values out of the `KeyValue` blob of a chat member.
* **Flag combinations are collapsed to one flag, with the raw mask kept alongside.**
  `persona_state_flag_from_bits`, `client_persona_state_flag_from_bits` and `account_flags_from_bits`
  take a raw bit set and, because the generated Rust enums cannot hold a combination, return the
  **single most specific** matching member (scanning the `ALL` arrays in declaration order). The raw
  bits are exposed as `status_flags_raw`, `state_flags_raw` and `account_flags_raw`.
* **`ChatMemberInfo` wraps a `MessageObject`** and reads itself out of the payload stream by
  hand (`read_from_stream`), advancing the `MsgBase` position by the number of bytes the
  reader actually consumed. `ChatEnterCallback` reads the room name first — the C# comment
  says steamclient always reads it, regardless of the enter response — and only reads the
  member list when the enter response is `Success`.

---

#### `crates/steamkit/src/steam/handlers/steam_friends/friend_cache.rs` — 176 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamFriends/FriendCache.cs`
**Purpose:** The private cache of every account this client knows about — the local user, the
known user accounts and the known clan accounts. It is what the `SteamFriends` getters read
from and what the persona-state and friends-list handlers write into.
**How it works:** Three structures:

* **`User`** — the cached state of a user account: `steam_id`, `name`, `avatar_hash`,
  `relationship` (`EFriendRelationship`), `persona_state` (`EPersonaState`),
  `persona_state_flags` (the raw bit pattern), `game_app_id`, `game_id`, `game_name`.
* **`Clan`** — the cached state of a clan account: `steam_id`, `name`, `avatar_hash`,
  `relationship` (`EClanRelationship`).
* **`AccountCache`** — the cache itself: `local_user: RwLock<User>`, `users:
  DashMap<SteamID, User>`, `clans: DashMap<SteamID, Clan>`.

The API is deliberately split into readers and writers:

| Method | Behaviour |
|---|---|
| `is_local_user(steam_id)` | compares against the local user's SteamID |
| `local_user()` | a **clone** of the local user |
| `user(steam_id)` | a clone of the cached user; **creates the entry if it is unknown** |
| `with_user(steam_id, f)` | mutates the cached user in place; **creates the entry if unknown** |
| `clan(steam_id)` / `with_clan(steam_id, f)` | the same for clans |
| `set_local_user_name`, `set_local_user_steam_id`, `set_local_user_persona_state` | the local-user setters |
| `local_user_persona_state()` | the local persona state |

**Notes and gotchas:** The C# has an abstract `Account` base class with sealed `User` and
`Clan` subclasses, and an `AccountList<T> : ConcurrentDictionary<SteamID, T>` whose
`GetAccount` is a `GetOrAdd`. Rust has no class inheritance, so the base class is flattened
into the two concrete structs and `AccountList<T>` becomes a plain `DashMap` with the
`GetOrAdd` behaviour written out at each call site. **A read mutates the cache**: `user(id)` and
`clan(id)` insert a default entry when the id is unknown, exactly like the C# `GetOrAdd`. That
means a lookup for an arbitrary SteamID grows the cache — worth knowing if you iterate over
unknown ids. The local user is special-cased on **both** sides of the API: `user(id)` returns
the local user's snapshot when `id` matches, and `with_user` mutates the local user instead of
the map. The local user's SteamID is only known once a `ClientFriendsList` arrives
(`set_local_user_steam_id`), so `is_local_user` returns `false` for everything until then.

---

#### `crates/steamkit/src/steam/handlers/steam_friends/steam_friends.rs` — 795 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamFriends/SteamFriends.cs`
**Purpose:** Everything that involves other people on the Steam network: the friend and clan
lists, the local persona name and state, friend requests, direct messages, and chat rooms
(joining, leaving, talking, kicking, banning, inviting).
**How it works:** `pub struct SteamFriends { client: Weak<SteamClient>, lists:
RwLock<FriendLists>, cache: AccountCache }`, where `FriendLists` holds the ordered
`friend_list: Vec<SteamID>` and `clan_list: Vec<SteamID>`. The read API is a set of getters
over the two structures: `get_persona_name`, `get_persona_state`, `get_friend_count`,
`get_friend_by_index`, `get_friend_persona_name`, `get_friend_persona_state`,
`get_friend_relationship`, `get_friend_game_played_name`, `get_friend_game_played`,
`get_friend_avatar`, `get_clan_count`, `get_clan_by_index`, `get_clan_name`,
`get_clan_relationship`, `get_clan_avatar`. `get_friend_by_index` and `get_clan_by_index`
return a zero SteamID when the index is out of range, mirroring the C#.

The write API is a set of one-shot sends: `set_persona_name`, `set_persona_state`,
`send_chat_message`, `add_friend`, `add_friend_by_steam_id`, `remove_friend`, `join_chat`,
`leave_chat`, `send_chat_room_message`, `invite_user_to_chat`, `kick_chat_member`,
`ban_chat_member`, `unban_chat_member`, `request_friend_info`,
`request_friend_info_for_steam_id`, `ignore_friend`, `request_profile_info`,
`request_message_history`, `request_offline_messages`. `kick`/`ban`/`unban` all funnel into
the private `chat_action(steam_id_chat, steam_id_member, action)`. `request_friend_info` and `request_friend_info_for_steam_id` take the requested `EClientPersonaStateFlag` mask as `Option<u32>`, so combined flags survive.

`handle_msg` has **two stages**. The first matches twelve `EMsg` values and posts the matching
callback. If none matched, the second stage handles five messages that need access to the
client or to the cache:

* `ClientPersonaState` → `handle_persona_state`, which updates the cache (by flag: player name
  → name, presence → avatar hash + persona state + state flags, game data blob → game name,
  game id and app id) and posts one `PersonaStateCallback` per friend entry.
* `ClientFriendsList` → `handle_friends_list`, which sets the local user's SteamID, clears the
  lists on a non-incremental update, classifies every entry as an individual or a clan,
  updates the relationships in the cache, removes the friends and clans whose relationship
  dropped to `None` (and the clans that were `Kicked`), and — because Steam only sends persona
  information for online friends — sends a `ClientRequestFriendData` for the whole list when
  the update is not incremental.
* `ClientChatGetFriendMessageHistoryResponse` → posts a `FriendMsgHistoryCallback`.
* `ClientAccountInfo` → caches the local persona name (this is why `SteamFriends` must be
  registered before `SteamUser`).
* `ClientPersonaChangeResponse` → updates the cached local persona name to what Steam says it
  is, and posts a `PersonaChangeCallback`.

**Notes and gotchas:** The chat-id fixup is the subtle part. A clan SteamID is rewritten into
its group-chat id by `to_chat_steam_id`: the account instance is cleared, `CHAT_INSTANCE_FLAG_CLAN
(0x0008_0000)` is set in the instance field, and the account type becomes `EAccountType::Chat`.
`join_chat` uses `SteamID::to_chat_id()` for a clan account and the raw id otherwise, while
`leave_chat`, `send_chat_room_message`, `invite_user_to_chat` and `chat_action` all use
`to_chat_steam_id`. `set_persona_name` writes the name into the cache **before** sending, so
that an early `set_persona_state` does not overwrite it with the stale name — the C# has the
same comment. `leave_chat` writes three values into the message payload by hand
(`ChatterActedOn`, `StateChange`, `ChatterActedBy`) because the chat leave message carries no
protobuf body for them. `request_offline_messages` also marks the messages as read server
side.

---

#### `crates/steamkit/src/steam/handlers/steam_game_coordinator/callbacks.rs` — 165 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamGameCoordinator/Callbacks.cs`
**Purpose:** The single callback of the game-coordinator handler: a message that arrived from a
game's coordinator.
**How it works:** `MessageCallback` carries the raw GC message type (`e_msg`, with the protobuf
flag still on it), the app id of the game coordinator it came from, and the message itself.
The message is the enum `PacketGCMsg` with two variants — `Protobuf(PacketClientGCMsgProtobuf)`
and `Plain(PacketClientGCMsg)` — because the C# stores an `IPacketGCMsg` interface value and
Rust has no interface-to-union conversion. `PacketGCMsg` implements `IPacketGCMsg` by
dispatching every method to the variant it holds.
**Notes and gotchas:** `get_packet_gc_msg(e_msg, data)` strips the protobuf flag with
`get_gc_msg(e_msg)` and then decides the variant with `is_proto_buf(e_msg)`. `MessageCallback::e_msg()`
returns the **stripped** message type, while `is_proto()` reports the flag; the raw value is
kept internally so both can be answered. `MessageCallback` is the only callback in this tree
that is **not** `Clone` — it wraps packet objects, not plain data.

---

#### `crates/steamkit/src/steam/handlers/steam_game_coordinator/steam_game_coordinator.rs` — 77 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamGameCoordinator/SteamGameCoordinator.cs`
**Purpose:** Sends messages to a game's coordinator and posts the messages that come back.
**How it works:** `pub struct SteamGameCoordinator { client: Weak<SteamClient> }` with one
public method, `send(&self, msg: &dyn IClientGCMsg, app_id: u32) -> SteamResult<()>`. It
wraps the GC message in a `CMsgGcClient` protobuf sent as `EMsg::ClientToGC`, setting the
protobuf header's `routing_appid` to the app id, the body's `msgtype` to
`make_gc_msg(msg.msg_type(), msg.is_proto())`, the body's `appid` to the app id, and the
body's `payload` to the serialised GC message. `handle_msg` matches `EMsg::ClientFromGC` and
posts a `MessageCallback`.
**Notes and gotchas:** The `routing_appid` is what makes Steam route the message to the right
game coordinator; without it the message goes nowhere. `make_gc_msg` folds the protobuf flag
into the message type in the same way the C# does, which is why `MessageCallback` has to strip
it again on the way back.

---

#### `crates/steamkit/src/steam/handlers/steam_game_server/callbacks.rs` — 141 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamGameServer/Callbacks.cs`
**Purpose:** The two callbacks of the game-server handler: the reply to a status update and
the completion of a ticket authentication.
**How it works:** `StatusReplyCallback` carries a single flag, `is_secure` — whether the game
server is VAC-secure. `TicketAuthCallback` carries the SteamID the ticket auth completed for,
the game id, the authentication state, the auth session response, the ticket CRC and the ticket
sequence.
**Notes and gotchas:** `TicketAuthCallback::auth_session_response()` returns
`Option<EAuthSessionResponse>` — the C# casts the raw value to the enum, which can produce a
value that is not a member of the enumeration; the Rust port returns `None` for such values and
exposes the raw number through `auth_session_response_value()`. This callback is the
game-server twin of `SteamAuthTicket`'s `TicketAuthCompleteCallback`: both are built from the
same `EMsg::ClientTicketAuthComplete` message, and a client that runs both handlers receives
both callbacks.

---

#### `crates/steamkit/src/steam/handlers/steam_game_server/steam_game_server.rs` — 258 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamGameServer/SteamGameServer.cs`
**Purpose:** Logs a client onto the Steam network **as a game server** (a persistent or
anonymous game server) and reports the status of that game server to Steam.
**How it works:** `pub struct SteamGameServer { client: Weak<SteamClient> }` with the
request-side types `LogOnDetails { token: Option<String>, app_id: u32 }` and `StatusDetails {
app_id, server_flags: u32 (an `EServerFlags` mask), game_directory, address: Option<IpAddr>, port, query_port,
version }`. `log_on(details)` requires a non-empty token, refuses to log on when the client is
not connected (it posts a `LoggedOnCallback` with `EResult::NoConnection` instead, exactly as
the C# does), builds a SteamID with `EAccountType::GameServer`, and sends it as
`EMsg::ClientLogonGameServer`. `log_on_anonymous(app_id)` does the same with
`EAccountType::AnonGameServer` and `EMsg::ClientLogon`. `log_off()` sets the client's
`expect_disconnection` flag and sends `EMsg::ClientLogOff`. `send_status(details)` sends a
`CMsgGsServerType` as `EMsg::GSServerType` with the app id served, the server flags, the game
directory, the game port, the query port, the game version and the deprecated game IP address.
**Notes and gotchas:** Two details are explicit in the source and easy to get wrong:

* **The SteamID is written raw.** `logon.proto_header_mut().steamid = Some(gs_id.convert_to_u64())`
  — the generic `set_steam_id` helper is deliberately *not* used, because it rebuilds the id
  from its parts and would apply the desktop account instance, while a game-server id carries
  an account instance of 0.
* **The private IP is obfuscated through `NetHelpers`.** `NetHelpers::get_msg_ip_address` then
  `NetHelpers::obfuscate_private_ip` — the same transformation `SteamUser` performs, but the
  shared helper is used here instead of the local copy.
* `send_status` rejects an IPv6 address with
  `SteamError::InvalidArgument("Only IPv4 addresses are supported.")`, because the wire field is
  a 32-bit integer. `handle_msg` matches `EMsg::GSStatusReply` and
  `EMsg::ClientTicketAuthComplete`.

---

#### `crates/steamkit/src/steam/handlers/steam_master_server/callbacks.rs` — 103 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamMasterServer/Callbacks.cs`
**Purpose:** The result of a master-server query: the list of game servers.
**How it works:** `QueryCallback` carries the job id and a `Vec<Server>`. `Server` carries a
`SocketAddr` (the IP endpoint and the query port) and the number of Steam-authenticated
players on that server.
**Notes and gotchas:** The C# **dereferences** the server's IP address, which throws when Steam
does not send one. The Rust port keeps the entry and substitutes the unspecified address
(`IpAddr::V4(Ipv4Addr::UNSPECIFIED)`). The same relaxation is applied in
`SteamMasterServer`'s module documentation, so it is a deliberate, recorded difference rather
than an accident. The query port is narrowed with `u16::try_from(...).unwrap_or(0)`.

---

#### `crates/steamkit/src/steam/handlers/steam_master_server/steam_master_server.rs` — 128 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamMasterServer/SteamMasterServer.cs`
**Purpose:** Requests a server list from the Steam game master server.
**How it works:** `pub struct SteamMasterServer { client: Weak<SteamClient> }` with the
request-side type `QueryDetails { app_id, filter: Option<String>, region: ERegionCode,
geo_located_ip: Option<IpAddr>, max_servers: u32 }`. The single public method
`server_query(&self, details) -> SteamResult<QueryCallback>` allocates a job id, starts the
job, fills a `CMsgClientGmsServerQuery` with the app id, the optional GeoIP address (through
`NetHelpers::get_ip_address_as_uint`), the filter text, the region code and the maximum
number of servers, sends it as `EMsg::ClientGMSServerQuery`, and awaits the job.
`handle_msg` matches `EMsg::GMSClientServerQueryResponse`.
**Notes and gotchas:** The C# `ServerQuery` returns an `AsyncJob<QueryCallback>`; the Rust port
returns `async fn -> Result<QueryCallback, SteamError>` per the porting contract §5.2. The
`filter` field is the Valve master-server query filter string (see the Master Server Query
Protocol documentation linked in the source). The default `region` is `ERegionCode::USEast`,
mirroring the C# default of the enumeration.

#### `crates/steamkit/src/steam/handlers/steam_matchmaking/callbacks.rs` — 516 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamMatchmaking/Callbacks.cs`
**Purpose:** The nine callbacks of the matchmaking handler, one per lobby operation plus the
two unsolicited notifications.
**How it works:** All nine follow the same compact pattern — a `base: CallbackMsgBase` for the
job id, a `pub(crate) fn new(job_id, …)` constructor, accessors, and an `impl CallbackMsg`:

* **`GetLobbyListCallback`** — app id, result, and the `Vec<Lobby>` that matched the query.
* **`CreateLobbyCallback`** — app id, result and the SteamID of the new lobby.
* **`SetLobbyDataCallback`** — app id, result and the lobby SteamID. The same callback answers
  both `set_lobby_data` and `set_lobby_member_data`.
* **`SetLobbyOwnerCallback`** — app id, result and the lobby SteamID.
* **`JoinLobbyCallback`** — app id, the `EChatRoomEnterResponse` and `Option<&Lobby>`. The
  lobby is `None` unless the chat-room enter response was `Success`.
* **`LeaveLobbyCallback`** — app id, result and the lobby SteamID.
* **`LobbyDataCallback`** — app id and the updated `Lobby`. It fires both as the answer to
  `get_lobby_data` and whenever Steam pushes new lobby data.
* **`UserJoinedLobbyCallback`** / **`UserLeftLobbyCallback`** — app id, lobby SteamID and the
  `Member` that joined or left. These are the only two callbacks in this file that are **not**
  the answer to a request: they are created with `CallbackMsgBase::new()`, so their job id is
  `JobID::INVALID` and they match no job.
**Notes and gotchas:** The `Lobby` and `Member` types live in `lobby.rs`, not here — this file
only carries the callback wrappers. Note the asymmetry between the callbacks that carry a
`result: EResult` (`Create`, `SetData`, `SetOwner`, `Leave`, `GetLobbyList`) and `JoinLobby`,
which carries an `EChatRoomEnterResponse` instead: joining a lobby is a chat-room operation,
and the chat room enter response is the meaningful outcome.

---

#### `crates/steamkit/src/steam/handlers/steam_matchmaking/lobby_cache.rs` — 172 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamMatchmaking/LobbyCache.cs`
**Purpose:** The cache of the lobbies this client knows about, grouped by app id and then by
the SteamID of the lobby. It is what `SteamMatchmaking::get_lobby` reads from and what the
lobby handlers keep up to date.
**How it works:** The cache is two levels deep:

```rust
pub struct LobbyCache {
    lobbies: DashMap<u32, Arc<DashMap<SteamID, Lobby>>>,   // app id → lobby id → lobby
}
```

The inner map is behind an `Arc` on purpose: `app_lobbies(app_id)` clones the `Arc` out of the
outer map and returns it, so a lookup never has to hold two shard locks at the same time and
cannot deadlock against a concurrent insert into the outer map. The C# uses a nested
`ConcurrentDictionary` with the same effect but without the explicit `Arc`.

The public API:

| Method | Behaviour |
|---|---|
| `get_lobby(app_id, lobby_steam_id)` | a clone of the cached lobby, or `None` |
| `cache_lobby(app_id, lobby)` | insert or replace |
| `add_lobby_member(app_id, lobby, member_id, persona_name)` | appends a member; returns `None` when the member is already in the lobby |
| `remove_lobby_member(app_id, lobby, member_id)` | removes a member; returns `None` when the member is not in the lobby |
| `clear_lobby_members(app_id, lobby_steam_id)` | empties the member list |
| `update_lobby_owner(app_id, lobby_steam_id, owner_steam_id)` | replaces the owner |
| `update_lobby_members(app_id, lobby, members)` | replaces the member list, keeping the owner |
| `clear()` | drops every cached lobby |

**Notes and gotchas:** **`Lobby` is an immutable value type**, so every mutation rebuilds it
and re-inserts it; `update_lobby_members_with_owner` is the single place that does the
reconstruction, and it copies the id, type, flags, metadata, max members, member count,
distance and weight across unchanged. The member list is a `Vec<Member>`, not a set, so
removal is a linear filter — the lists are small. `remove_lobby_member` **deletes the lobby
entirely when the last member leaves**, because Steam deletes lobbies that contain no
members; the C# does the same, with the same comment. One behavioural difference from the C#:
the C# `ClearLobbyMembers` calls `UpdateLobbyMembers(appId, lobby, null, null)` — it clears the
owner as well as the members — whereas the Rust `clear_lobby_members` passes
`lobby.owner_steam_id()` and therefore **keeps the owner** while clearing the members. The
members-only clear is what `SteamMatchmaking::handle_leave_lobby_response` and
`handle_user_left_lobby` rely on.

---

#### `crates/steamkit/src/steam/handlers/steam_matchmaking/lobby.rs` — 480 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamMatchmaking/Lobby.cs`
**Purpose:** The value types of the lobby protocol: the lobby itself, its members, and the five
kinds of filter that can narrow a lobby-list query.
**How it works:** Three groups.

* **`Lobby`** — `steam_id`, `lobby_type: ELobbyType`, `lobby_flags: i32`,
  `owner_steam_id: Option<SteamID>`, `metadata: BTreeMap<String, String>`, `max_members`,
  `num_members`, `members: Vec<Member>`, `distance: Option<f32>`, `weight: Option<i64>`. The
  constructor is crate-private (`pub(crate) fn new`) — the C#'s is `internal` too — because a
  lobby is only ever created by the handler or the cache. `owner_steam_id` is `Option` because
  Steam does not send owner details for lobbies returned in a lobby list.
* **`Member`** — `steam_id`, `persona_name`, `metadata: BTreeMap<String, String>`.
* **`Filter`** — a **trait** (the C# `Lobby.Filter` is an abstract class) with five
  implementations: `DistanceFilter` (geographic, by `ELobbyDistanceFilter`), `NearValueFilter`
  (metadata value closest to an integer), `NumericalFilter` (integer comparison against a
  metadata value, with an `ELobbyComparison`), `SlotsAvailableFilter` (minimum free slots) and
  `StringFilter` (string comparison against a metadata value). `DistanceFilter` and
  `SlotsAvailableFilter` have an **empty key** — under some circumstances a filter has no
  metadata key at all. Every filter implements `serialize()` into the protobuf
  `c_msg_client_mms_get_lobby_list::Filter`, which carries the key, the value **as a
  string**, the comparison and the filter type.

The metadata codec is the interesting part: `encode_metadata` turns a `BTreeMap` into a
`Vec<MmsKeyValuePair>`, and `decode_metadata(pairs, legacy_buffer)` prefers the key-value pair
list and falls back to the **legacy binary `KeyValue` blob** when the list is empty.
`decode_metadata_buffer` parses that blob with `KeyValue::try_read_as_binary`.
**Notes and gotchas:** The C# metadata dictionaries are `IReadOnlyDictionary<string, string>`;
the Rust port uses `BTreeMap` so that the iteration order — and with it the order of the
encoded metadata pairs — is deterministic. The C# `Lobby.DecodeMetadata(byte[])` throws a
`FormatException` when the legacy blob cannot be parsed; the Rust port returns
`SteamError::Malformed("Lobby metadata is of an unexpected format")`. `Member` implements
`PartialEq`, `Eq` and `Hash` **by SteamID only**, mirroring the C# `Member.Equals` override
that ignores the persona name and the metadata.

---

#### `crates/steamkit/src/steam/handlers/steam_matchmaking/steam_matchmaking.rs` — 995 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamMatchmaking/SteamMatchmaking.cs`
**Purpose:** Creating, joining, leaving, listing and updating Steam lobbies — the modern
matchmaking service (`ClientMMS*` messages).
**How it works:** `pub struct SteamMatchmaking { client: Weak<SteamClient>,
lobby_manipulation_requests: DashMap<JobID, LobbyManipulationRequest>, lobby_cache:
LobbyCache }`.

The public API is eight async methods plus one sync one:

* `create_lobby(app_id, lobby_type, max_members, lobby_flags, metadata)` — needs the client's
  cell id and the local persona name (from the `SteamFriends` handler), sends
  `ClientMMSCreateLobby` with the public IP and the encoded metadata.
* `set_lobby_data(app_id, lobby_steam_id, lobby_type, max_members, lobby_flags, metadata)` and
  `set_lobby_member_data(app_id, lobby_steam_id, metadata)` — both send `ClientMMSSetLobbyData`;
  the first sets `steam_id_member = 0` (the lobby itself), the second sets it to the local
  SteamID (one member's metadata).
* `set_lobby_owner(app_id, lobby_steam_id, new_owner)` — sends `ClientMMSSetLobbyOwner`.
* `get_lobby_list(app_id, filters, max_lobbies)` — sends `ClientMMSGetLobbyList` with the
  filters serialised and `num_lobbies_requested = max_lobbies` (`-1` means "no limit").
* `join_lobby(app_id, lobby_steam_id)` — needs the local persona name, sends
  `ClientMMSJoinLobby`.
* `leave_lobby(app_id, lobby_steam_id)` — sends `ClientMMSLeaveLobby`.
* `get_lobby_data(app_id, lobby_steam_id)` — sends `ClientMMSGetLobbyData`.
* `invite_to_lobby(app_id, lobby_steam_id, user_steam_id)` — sends `ClientMMSInviteToLobby` and
  returns immediately: Steam provides no way to tell whether the invite succeeded.
* `get_lobby(app_id, lobby_steam_id) -> Option<Lobby>` — a pure cache lookup, no network.

Every one of them goes through the private `send(&self, msg, app_id)`, which sets
`proto_header_mut().routing_appid = app_id` — **without it Steam does not route the response
back**. `handle_msg` dispatches nine `EMsg` values to nine private handlers, each of which
updates the cache and posts the matching callback.
**Notes and gotchas:**

* **The pending-manipulation table.** The C# stores the request body in a
  `ConcurrentDictionary<JobID, IExtensible>` and casts it back to the concrete protobuf type
  when the response arrives. Rust cannot downcast a trait object back to a concrete type, so
  the port uses the enum `LobbyManipulationRequest { CreateLobby(..), SetLobbyData(..),
  SetLobbyOwner(..) }` — the same information, without the cast.
* **The table is the reason the responses can update the cache.** When the create/set-data/
  set-owner response arrives, the handler removes the entry and uses the stored request to
  rebuild the lobby in the cache (the response body alone does not carry the metadata).
* **The pending entry is removed on failure and on cancellation.** `finish_manipulation` removes it
  when the job did not complete successfully, and a `ManipulationGuard` created before the await
  removes it if the caller future is dropped, so a timed-out or abandoned request cannot leak.
* **The lobby cache is cleared on disconnect.** `SteamClient::clear_handler_caches` calls
  `SteamMatchmaking::clear_lobby_cache` when the connection drops.
* `get_lobby_list` preserves what the cache already knows: the members and the owner of an
  already-known lobby are carried over into the new `Lobby`, and the lobbies from the list are
  written back into the cache.
* `handle_user_joined_lobby` and `handle_user_left_lobby` do nothing when the lobby is unknown
  **or when its member list is empty** — the C# has the same `lobby != null && lobby.Members.Count
  > 0` guard. When the member who leaves is the local user, the handler additionally clears the
  lobby's members.

#### `crates/steamkit/src/steam/handlers/steam_networking/callbacks.rs` — 71 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamNetworking/Callbacks.cs`
**Purpose:** The result of a networking-certificate request.
**How it works:** `NetworkingCertificateCallback` carries the certificate signed by the Steam
CA, the id of the CA key that signed it, and the signature that verifies the certificate. The
certificate contains a `CMsgSteamDatagramCertificate` with the public key that was submitted.
**Notes and gotchas:** The job id comes from `packet_msg.target_job_id()`, so this callback is
the answer to a specific request and can be matched with `subscribe_job`. The certificate is
returned as raw bytes — the Rust port does not decode the inner
`CMsgSteamDatagramCertificate`, exactly like the C#.

---

#### `crates/steamkit/src/steam/handlers/steam_networking/steam_networking.rs` — 85 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamNetworking/SteamNetworking.cs`
**Purpose:** Requests a signed networking certificate for an Ed25519 public key, which is what
a Steam Datagram socket needs to authenticate itself.
**How it works:** `pub struct SteamNetworking { client: Weak<SteamClient> }` with one public
method, `request_networking_certificate(app_id, public_key) -> SteamResult<
NetworkingCertificateCallback>`. It allocates a job id, starts the job, fills a
`CMsgClientNetworkingCertRequest` with the app id and the key bytes, sends it as
`EMsg::ClientNetworkingCertRequest`, and awaits the job. `handle_msg` matches
`EMsg::ClientNetworkingCertRequestResponse`.
**Notes and gotchas:** The C# method returns an `AsyncJob<NetworkingCertificateCallback>`; per
the porting contract §5.2 the Rust port returns the callback itself. The public key is passed
as a byte slice and copied verbatim into the protobuf.

---

#### `crates/steamkit/src/steam/handlers/steam_screenshots/callbacks.rs` — 64 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamScreenshots/Callbacks.cs`
**Purpose:** The result of adding a screenshot to the user's screenshot library.
**How it works:** `ScreenshotAddedCallback` carries the `EResult` and the id of the new
screenshot, as a `UGCHandle`.
**Notes and gotchas:** The job id comes from `packet_msg.target_job_id()`, so this is the
answer to one specific `add_screenshot` call.

---

#### `crates/steamkit/src/steam/handlers/steam_screenshots/steam_screenshots.rs` — 157 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamScreenshots/SteamScreenshots.cs`
**Purpose:** Adds a screenshot to the user's screenshot library. The image and its thumbnail
must already exist on the UFS — this handler only registers them.
**How it works:** `pub struct SteamScreenshots { client: Weak<SteamClient> }` with the
request-side type `ScreenshotDetails { game_id: Option<GameID>, ufs_image_file_path,
ufs_thumbnail_file_path, caption, privacy: EUCMFilePrivacyState, width, height, creation_time:
DateTime<Utc>, contains_spoilers }`. `add_screenshot(details)` allocates a job id, starts the
job, fills a `CMsgClientUcmAddScreenshot` with the app id (taken from the `GameID`), the
caption, the UFS image file name, the thumbnail name, the width, the height, the
`rtime32_created` timestamp and the spoiler flag, sends it as
`EMsg::ClientUCMAddScreenshot`, and awaits the job. `handle_msg` matches
`EMsg::ClientUCMAddScreenshotResponse`.
**Notes and gotchas:** Two defaults are deliberate. The C#'s default `Privacy` value is the
**unnamed** enum member 0, which has no counterpart in the generated enumeration; the Rust port
uses `EUCMFilePrivacyState::Invalid` and then does not send the field at all — because a
protobuf field that carries its default value is left out of the message entirely, which is
exactly what the C# does with the zero value. The C#'s default `CreationTime` is
`DateTime.MinValue`; the Rust port uses the unix epoch instead, so that `rtime32_created`
exactly what the C# does with the zero value. The C# default `CreationTime` is
`DateTime.MinValue`, whose negative unix time truncates to `2_288_912_640` in a `uint`; the Rust
port uses that same value, so `rtime32_created` matches the C#. `SCREENSHOT_THUMBNAIL_WIDTH = 200` is exported for callers that generate the thumbnail.
---

#### `crates/steamkit/src/steam/handlers/steam_unified_messages/callbacks.rs` — 152 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamUnifiedMessages/Callbacks.cs`
**Purpose:** The two callback shapes of the unified-messages service: the response to a
service method call, and an unsolicited service notification.
**How it works:** Both are generic over the protobuf body type:

* **`ServiceMethodResponse<T>`** — the job id, the `EResult` of the call, and the decoded body
  `T`. `from_packet` reads the protobuf header (`jobid_target` for the job id, `eresult` for
  the result) and the body; `from_parts` decodes a body from raw bytes, which is what the
  unified-messages handler uses after it has already parsed the header itself.
* **`ServiceMethodNotification<T>`** — the job id, the `job_name` (`Service.Method#Version`)
  and the decoded body `T`.

**Notes and gotchas:** A notification carries `JobID::INVALID` — it is not the answer to a
request, so it must never complete a job. `ServiceMethodResponse::from_packet` reads the job
id from `jobid_target` and falls back to `0` when the field is absent, matching the C# `ulong`
default. Both types are `Clone`, so the job manager can clone them for a waiting job.
job, as it does for every other callback.

---

#### `crates/steamkit/src/steam/handlers/steam_unified_messages/steam_unified_messages.rs` — 478 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamUnifiedMessages/SteamUnifiedMessages.cs`
**Purpose:** The transport of the Steamworks unified-messaging service — the RPC mechanism that
most modern Steam services (Cloud, ContentServerDirectory, the friends and store services) are
built on.
**How it works:** Two types.

* **`SteamUnifiedMessages`** - the handler. It holds `services: DashMap<String,
Arc<UnifiedService>>` (the registered services by name) and two decoder tables,
`response_decoders` and `notification_decoders`, keyed by `(service, method)`. `create_service(name)`
uses `entry().or_insert_with`, so concurrent first use yields one service; `remove_service(name)`
drops the service but keeps the decoders, so creating it again resumes delivery, like the C#
generated switches. `send_message(name, message)` allocates a job id, starts a job for
`ServiceMethodResponse<TResult>`, writes the RPC name into the protobuf header's `target_job_name`
(`Service.Method#1`), sends it as `EMsg::ServiceMethodCallFromClient` (or `.NonAuthed` when the
client has no SteamID yet), and returns the `AsyncJob` - it does not await it, like the C#.
`send_notification(name, message)` sends the same way but **without** a source job id - Steam
actively rejects a notification that carries one.
* **`UnifiedService`** - a handle on one service. It holds the service name and a
  `Weak<SteamClient>`; the decoder tables live on the handler, keyed by service and method, so they
  survive a remove/recreate. `set_response_type::<T>(method)` and
  `set_notification_type::<T>(method)` declare the protobuf body type of a method; a second
  registration with a different type is rejected (the first wins). `send_message::<TRequest,
  TResult>(method, message)` registers `TResult` and returns the `AsyncJob`.
`handle_msg` matches `EMsg::ServiceMethodResponse` and `EMsg::ServiceMethod`, both of which
are ignored unless the packet is a protobuf. `read_service_message` parses the
`MsgHdrProtoBuf` header, takes the rest of the packet as the raw body, and splits the job name
into a service name and a method name at the first `.` and the last `#`.
**Notes and gotchas:**

* **The C# `UnifiedService` is an abstract class** that the generated service clients derive
  from; `prost` does not generate those clients, so the Rust port uses a concrete struct whose
  body types are declared at runtime. This is why `SteamCloud` and `SteamContent` ask for a
  service **by name**.
* **A response is dispatched only to a registered service.** The handler looks the service up by
  name and decodes with the `(service, method)` decoder; if the service is not registered the
  message is dropped and the job times out, exactly like the C#. There is no `pending` map: the
  decoder registered by `send_message` (or by `subscribe_service_*`) is the one used.
* `service_method_name(service, method)` builds the wire name `Service.Method#1`
  (`SERVICE_METHOD_VERSION = 1`).
* The service handle holds a `Weak<SteamClient>` and reaches the handler through
  `client.get_required_handler::<SteamUnifiedMessages>()`.

---

#### `crates/steamkit/src/steam/handlers/steam_user/callbacks.rs` — 852 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamUser/Callbacks.cs`
**Purpose:** The callback family of the user handler — the ten messages that describe the
logged-on account and its session.
**How it works:** Ten callbacks, each with a `pub(crate) fn from_packet(&dyn IPacketMsg) ->
SteamResult<Self>`:

* **`LoggedOnCallback`** — the answer to `log_on`. Carries the `EResult` and the extended
  result, the out-of-game and in-game heartbeat intervals, the public IP, the server time, the
  account flags, the client SteamID, the email domain, the Steam2 cell id and its ping
  threshold, the Steam2 ticket, the IP and user country codes, the vanity URL, the two
  migration thresholds (`num_login_failures_to_migrate`, `num_disconnects_to_migrate`) and the
  parental settings. `from_result(result)` builds the variant used when the log-on never
  reached the server.
* **`LoggedOffCallback`** — the server told us to log off; carries the `EResult`.
* **`SessionTokenCallback`** — the unique Steam3 session token used for authenticated content
  downloads.
* **`AccountInfoCallback`** — the account information that arrives after logon: the persona
  name, the country, the number of SteamGuard-authenticated computers, the account flags.
* **`EmailAddrInfoCallback`** — the account's email address and whether it is validated.
* **`WalletInfoCallback`** — the wallet: whether there is one, the currency, the 32-bit
  balance and delayed balance, and the 64-bit variants.
* **`WebAPIUserNonceCallback`** — the result of a WebAPI user-nonce request and the nonce.
* **`VanityURLChangedCallback`** — the new vanity URL.
* **`MarketingMessageCallback`** — the marketing message update: the update time and a
  `Vec<MarketingMessage>` (id, URL, flags).
* **`PlayingSessionStateCallback`** — whether playing is blocked by another client, and which
  app is being played.
**Notes and gotchas:** **`LoggedOnCallback::from_packet` handles both wire formats.** It
checks `packet_msg.is_proto()`: the protobuf path fills every field, while the legacy
`MsgClientLogOnResponse` path fills only the subset that the old struct has (result, the two
heartbeat rates, the public IP, the server time and the client SteamID) and leaves the rest at
their defaults. `ip_address_from_msg` reproduces the C# `NetHelpers.GetIPAddress`: the v4 case
shuffles the bytes (`raw & 0xFF00_0000) >> 24` and so on), the v6 case requires exactly 16
bytes. The `MarketingMessageCallback` parses a **binary blob**: it reads `count` records, each
prefixed by a 4-byte length that *includes the length field itself*, so the payload length is
`read_i32() - 4`; each record is a `GlobalID`, a null-terminated URL and the flags. The job id
is `JobID::INVALID` for the callbacks that are not the answer to a request
(`LoggedOn`, `LoggedOff`, `SessionToken`, `AccountInfo`, `EmailAddrInfo`, `WalletInfo`,
`MarketingMessage`) and `packet_msg.target_job_id()` for the ones that are (`WebAPIUserNonce`,
`VanityURLChanged`, `PlayingSessionState`). `LoggedOnCallback` and `AccountInfoCallback` expose `account_flags_raw` alongside the typed `account_flags`, and `AccountInfoCallback::persona_name()` returns `Option<&str>` (a missing name stays `None`).

---

#### `crates/steamkit/src/steam/handlers/steam_user/steam_user.rs` — 450 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamUser/SteamUser.cs`
**Purpose:** **The most important handler in the set.** It performs the logon — sending the
`ClientLogon` message with the credentials, the Steam Guard codes, the access token and the
obfuscated private IP — and it turns the ten account messages that Steam sends afterwards into
callbacks.
**How it works:** `pub struct SteamUser { client: Weak<SteamClient> }` with the request-side
types `LogOnDetails` and `AnonymousLogOnDetails`, and the `ChatMode` enum (`Default = 0`,
`NewSteamChat = 2`). `LogOnDetails` mirrors the C# class field for field: `username`,
`password`, `cell_id`, `login_id`, `auth_code`, `two_factor_code`, `should_remember_password`,
`access_token`, `account_instance`, `account_id`, `request_steam2_ticket`, `client_os_type`,
`client_language`, `machine_name`, `chat_mode`, `ui_mode`, `gaming_device_type`. Its `Default`
uses the desktop account instance (1), `"english"`, the detected OS type, and a machine name of
`"{COMPUTERNAME} (SteamKit2)"`.

`log_on(details)` is the heart of the handler:

1. Validates that a username is present and that either a password or an access token is
   present; otherwise `SteamError::InvalidArgument`.
2. If the client is not connected, posts a `LoggedOnCallback` with `EResult::NoConnection` and
   returns `Ok(())` — the C# does the same.
3. Builds the SteamID from `account_id`, `account_instance`, the client's universe and
   `EAccountType::Individual` (packed by the local `pack_steam_id`, which lays out the fields
   the way Valve documents: account id in bits 0-31, instance in 32-51, type in 52-55, universe
   in 56-63).
4. Computes the obfuscated private IP: if `login_id` is set it is used **as the v4 value
   directly** (it is already the login id, not an IP to be obfuscated); otherwise the client's
   local IP is obfuscated by `obfuscate_private_ip`.
5. Fills the `CMsgClientLogon` body: account name, password, `should_remember_password`, the
   protocol version, the OS type, the language, the cell id (from the details or the
   configuration), the Steam2 ticket request, `client_package_version = 1771`,
   `supports_rate_limit_response = true`, the machine name, the machine id from the
   configuration's machine-info provider, the Steam Guard codes, the access token, and the
   chat/ui/gaming-device modes — each of the last three only when it differs from its default.
6. Sends it as `EMsg::ClientLogon`.

`log_on_anonymous()` and `log_on_anonymous_with_details(details)` build the same message with
an `EAccountType::AnonUser` id of all zeros and no credentials. `log_off()` sets the client's
`expect_disconnection` flag and sends `EMsg::ClientLogOff`; the server then closes the
connection and a `DisconnectedCallback` with `UserInitiated = true` is posted.
`steam_id()` returns the client's SteamID, which is only assigned after a successful logon.
`handle_msg` matches the ten `EMsg` values and posts the matching callback.
**Notes and gotchas:**

* **`client_package_version` is hard-coded to 1771**, with the C#'s comment: Steam requires a
  current package version to produce a proper sentry file for Steam Guard.
* **The IPv6 obfuscation XORs each 4-byte chunk with `0x0D 0xF0 0xAD 0xBA`.** The IPv4 case
  XORs the whole 32-bit value with `MsgClientLogon::OBFUSCATION_MASK`, keeping the bytes in
  network order (the C# `NetHelpers.GetIPAddressAsUInt`).
* **The deprecated field is set from the v4 value only.** `deprecated_obfustucated_private_ip`
  (the misspelling is Valve's) is filled when the obfuscated address is an IPv4 one.
* **`chat_mode`, `ui_mode` and `gaming_device_type` are omitted when they equal their
  default** — `ChatMode::Default`, `EUIMode::Unknown`, `EGamingDeviceType::Unknown` — exactly
  like the C#, which only assigns them inside an `if`.
* The C# `LogOn` throws `ArgumentNullException` for a null `details` and `ArgumentException`
  for a missing username/password; the Rust port takes `&LogOnDetails` (so null is
  impossible) and returns `SteamError::InvalidArgument` for the missing credentials.
* The `machine_name` helper reads the `COMPUTERNAME` environment variable, falls back to
  `HOSTNAME`, and finally to `"unknown"`.

---

#### `crates/steamkit/src/steam/handlers/steam_user_stats/callbacks.rs` — 278 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamUserStats/Callbacks.cs`
**Purpose:** The three callbacks of the statistics handler: the current player count, the
leaderboard descriptor and the leaderboard entries.
**How it works:** `NumberOfPlayersCallback` carries the result and the number of players.
`FindOrCreateLeaderboardCallback` carries the result, the leaderboard id, the number of
entries, the sort method and the display type. `LeaderboardEntriesCallback` carries the
result, the total entry count and a `Vec<LeaderboardEntry>`; each entry has the player's
SteamID, the global rank, the score, the attached `UGCHandle` and the game-defined `details`
(an opaque blob that the port splits into `i32` chunks).
**Notes and gotchas:** The `details` blob is game-defined; the C# exposes it as a `byte[]`
(the port splits it into little-endian `i32`s for convenience). All three callbacks take
their job id from `packet_msg.target_job_id()`, so each is the answer to exactly one request.

---

#### `crates/steamkit/src/steam/handlers/steam_user_stats/steam_user_stats.rs` — 216 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamUserStats/SteamUserStats.cs`
**Purpose:** The Steam user-statistics API: the number of players currently in an app, and the
leaderboards.
**How it works:** `pub struct SteamUserStats { client: Weak<SteamClient> }` with four public
methods, all `async fn` returning the callback directly: `get_number_of_current_players(app_id)`
(sends `EMsg::ClientGetNumberOfCurrentPlayersDP`), `find_leaderboard(app_id, name)` and
`create_leaderboard(app_id, name, sort_method, display_type)` (both send
`EMsg::ClientLBSFindOrCreateLB`, with `create_if_not_found` false and true respectively), and
`get_leaderboard_entries(app_id, id, range_start, range_end, data_request)` /
`download_leaderboard_entries_for_users(app_id, id, users)` (both send
`EMsg::ClientLBSGetLBEntries`). `handle_msg` matches the three corresponding response
`EMsg` values.
**Notes and gotchas:** **The leaderboard requests must set `routing_appid`.** The three
leaderboard methods write `msg.header_mut().proto.routing_appid = Some(app_id)` — the source
says so explicitly: without it, Steam does not deliver the response. `create_leaderboard` is
the only method that sends the sort method and the display type; `find_leaderboard` sends
neither, because a leaderboard that does not exist yet has no settings. The C# methods return
an `AsyncJob<T>`; per the porting contract §5 the Rust port returns the callback itself.

---

#### `crates/steamkit/src/steam/handlers/steam_workshop/callbacks.rs` — 121 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamWorkshop/Callbacks.cs`
**Purpose:** The result of enumerating the published files of the local user.
**How it works:** `UserActionPublishedFilesCallback` carries the `EResult`, the total number of
results and a `Vec<File>`; each `File` has a `PublishedFileID` and the timestamp at which the
file was published.
**Notes and gotchas:** The timestamp comes from `rtime_time_stamp`, which is a **64-bit**
unix timestamp here, converted with `DateTime::from_timestamp` and falling back to the epoch.
The job id comes from `packet_msg.target_job_id()`.

---

#### `crates/steamkit/src/steam/handlers/steam_workshop/steam_workshop.rs` — 110 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Handlers/SteamWorkshop/SteamWorkshop.cs`
**Purpose:** Requests the files the local user has published to the Steam Workshop.
**How it works:** `pub struct SteamWorkshop { client: Weak<SteamClient> }` with the
request-side type `EnumerationUserDetails { app_id, start_index, user_action:
EWorkshopFileAction }`. The single public method
`enumerate_published_files_by_user_action(details)` allocates a job id, starts the job, fills a
`CMsgClientUcmEnumeratePublishedFilesByUserAction` with the action, the app id and the start
index, sends it as `EMsg::ClientUCMEnumeratePublishedFilesByUserAction`, and awaits the job.
`handle_msg` matches `EMsg::ClientUCMEnumeratePublishedFilesByUserActionResponse`.
**Notes and gotchas:** The default `user_action` is `EWorkshopFileAction::Played`, mirroring
the C# default of the enumeration. The C# method returns an
`AsyncJob<UserActionPublishedFilesCallback>`; per the porting contract §5.2 the Rust port
returns the callback itself. The enumeration is paged: `start_index` selects the window and
the callback's `total_results` tells the caller how many entries exist in total.

### 5.8 `crates/steamkit/src/steam/discovery/` — finding the servers


### 5.9 `crates/steamkit/src/steam/cdn/` — the depot content CDN


### 5.10 `crates/steamkit/src/steam/webapi/` — the HTTP/JSON Web API


### 5.11 `crates/steamkit/src/steam/authentication/` — logging on


This section documents four self-contained subsystems of the Rust port of SteamKit2:

| Rust module | C# namespace | Responsibility |
|---|---|---|
| `crates/steamkit/src/steam/discovery/` | `SteamKit2.Discovery` | CM server list: discovery, persistence, scoring, candidate selection |
| `crates/steamkit/src/steam/cdn/` | `SteamKit2.CDN` | Steampipe content servers: manifest/chunk download, chunk decryption, Lancache |
| `crates/steamkit/src/steam/webapi/` | `SteamKit2` (WebAPI) | Steam Web API client, Steam Directory, content-server directory |
| `crates/steamkit/src/steam/authentication/` | `SteamKit2.Authentication` | QR and password login, Steam Guard, access tokens |

Conventions used throughout: the C# original lives under `csharp-original/SteamKit2/SteamKit2/Steam/…`; the C# class name is always given in PascalCase; the Rust type name is always given in `PascalCase` as well, so where they differ the difference is called out explicitly. The library forbids `unsafe`; every type in these four modules is safe Rust. C# `Task`/`TaskCompletionSource`/`CancellationToken` become tokio: `async fn`, `tokio::sync::oneshot`/`broadcast`/`Mutex`/`RwLock`, and `tokio_util::sync::CancellationToken`.

---

#### 5.1 `discovery/` — the CM server list

The discovery module answers one question: *which Steam connection manager (CM) server should the client try next?* In the C# it is seven classes under `SteamKit2.Discovery`. The Rust port keeps the same split but adds one file that has no direct C# class: `basic_server_list_proto.rs`, a hand-written protobuf codec that replaces `protobuf-net` so the persisted server list stays byte-compatible.

The flow is:

1. A **server list provider** (`IServerListProvider`) supplies a cached list of servers and persists refreshes.
2. The **Steam Directory** Web API (`ISteamDirectory/GetCMListForConnect`) supplies a fresh list.
3. `SmartCMServerList` merges the two, tracks the per-protocol quality of every entry, and hands out the next candidate.

#### `crates/steamkit/src/steam/discovery/mod.rs` — 9 lines
**C# counterpart:** none — the C# has no `Discovery.cs`; the namespace is implicit in C# and the module list is implicit in Rust.
**Purpose:** Declares the eight sibling modules of the `discovery` module tree. It contains nothing but `pub mod` re-exports.
**How it works:** `basic_server_list_proto`, `file_storage_server_list_provider`, `i_server_list_provider`, `isolated_storage_server_list_provider`, `memory_server_list_provider`, `null_server_list_provider`, `server_record`, `smart_cm_server_list`. Every module is public, so the whole subsystem is reachable as `steamkit::steam::discovery::*`.
**Notes and gotchas:** The C# marks `BasicServerListProto` as `internal`; the Rust equivalent is `pub(crate)` inside its own module, so it is not re-exported.

#### `crates/steamkit/src/steam/discovery/smart_cm_server_list.rs` — 899 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/SmartCMServerList.cs`
**Purpose:** Keeps the known CM servers together with the quality of each server/protocol combination, decides when the list has to be (re)fetched, and picks the next server candidate. It is the single place where the Steam Directory, the persisted provider list and the built-in fallback servers are reconciled.
**How it works:**

*Types.* `ServerQuality { Good, Bad }` mirrors the C# enum of the same name. `ServerInfo { record: ServerRecord, protocol: ProtocolTypes, last_bad_connection_time_utc: Option<SystemTime> }` is the C#'s private nested `ServerInfo` class; one `ServerInfo` exists per **(record, single protocol flag)** pair. `ServerListState { servers: Vec<ServerInfo>, servers_last_refresh: Option<SystemTime> }` is the C#'s `Collection<ServerInfo> servers` + `DateTime serversLastRefresh` pair, bundled so it can be guarded by one lock.

*Struct.* `SmartCMServerList` holds `provider: Arc<dyn IServerListProvider>`, `allow_directory_fetch: bool`, `directory_loader: Option<ServerListDirectoryLoader>`, `state: RwLock<ServerListState>` (a `parking_lot` lock — the critical sections never await), `fetch_state: Mutex<FetchState>` plus a `tokio::sync::watch` channel (the single-flight resolve: in-flight flag, generation and last outcome; waiters wake on completion), and two `AtomicU64` counters holding the two time spans in **milliseconds**: `server_list_before_refresh_time_span` (default 7 days) and `bad_connection_memory_time_span` (default 5 minutes).

*Constructors.* `SmartCMServerList::new(state: Arc<SteamConfigurationState>)` is the production constructor. It reads `server_list_provider`, `allow_directory_fetch`, `web_api_base_address`, `cell_id`, `web_api_key` and `http_client_factory` out of the configuration **state** and installs a directory loader. `with_provider(provider, allow_directory_fetch)` installs the default Steam Directory loader when `allow_directory_fetch` is true, so the flag means what it says; pass `false` for a provider-only list. `with_directory_loader(provider, allow_directory_fetch, loader)` installs a caller-supplied loader. `with_steam_directory(provider, allow_directory_fetch, cell_id)` is a convenience that uses the default Web API base address and no API key.

*The directory loader.* `ServerListDirectoryLoader` is `Arc<dyn Fn() -> BoxFuture<'static, Result<Vec<ServerRecord>, SteamError>> + Send + Sync>`. Two constructors exist:

- `steam_directory_loader(base_address, cell_id, api_key, http_client)` — takes an already-built `reqwest::Client` (or `None`, in which case a default client with a 10 s timeout, `DIRECTORY_REQUEST_TIMEOUT`, is built per call).
- `steam_directory_loader_with_factory(base_address, cell_id, api_key, factory)` — takes the configuration's `HttpClientFactory` and holds the client in an `Arc<OnceLock<reqwest::Client>>`. The factory is invoked through `client.get_or_init(|| factory(HttpClientPurpose::WebAPI))` **inside** the loader, so the user's HTTP client factory runs at most once and only when the server list is actually fetched for the first time.

This laziness is deliberate and was a real defect before it was fixed. The C# reaches the directory through `SteamDirectory.LoadAsync(configuration)` → `configuration.GetAsyncWebAPIInterface("ISteamDirectory")` → `HttpClientFactory(HttpClientPurpose.WebAPI)`, and the factory call happens at the moment the directory is *queried*, not when the configuration is built. An earlier Rust version invoked the factory while the `SteamConfiguration` was being constructed; the test suite caught it (§12 of this guide, defect 6) and the fix was to move the construction into the `OnceLock`. The observable consequence: **building a `SteamConfiguration` never calls the user's HTTP client factory**; only the first server-list fetch does.

`load_server_list_from_steam_directory(base_address, cell_id, api_key, http_client)` performs the actual query: it builds an `AsyncInterface` for the `ISteamDirectory` interface and calls `SteamDirectory::load_with_interface(&directory, cell_id, None, CancellationToken::new())`, which issues `ISteamDirectory/GetCMListForConnect` v1 in the `vdf` format.

*Resolution.* `resolve_server_list(force_refresh)` runs this sequence:

1. `provider.last_server_list_refresh()` gives the provider's timestamp.
2. If the call was not already forced, and the provider's timestamp is at least `server_list_before_refresh_time_span` old, the call is upgraded to a forced refresh.
3. If forced **and** `allow_directory_fetch` **and** a loader exists: query the Steam Directory. A non-empty result is installed with `replace_list_internal(list, write_provider = true, now)` and the function returns. An empty result falls through to the provider.
4. `provider.fetch_server_list()`: a non-empty result is installed with `replace_list_internal(list, write_provider = false, provider_refresh_time)` and the function returns. Note the C#'s asymmetry: the provider's list is *not* written back to the provider.
5. If `allow_directory_fetch` is false, the list is emptied (`replace_list_internal(vec![], false, UNIX_EPOCH)`) and the function returns.
6. If the directory was not tried in step 3, try it now; a non-empty result is installed with `write_provider = true`.
7. Last resort: the two built-in fallback servers, installed with `replace_list_internal(defaults, false, UNIX_EPOCH)`.

*Freshness and fetching.* `list_is_fresh(span)` is true when the list is non-empty **and** `now - servers_last_refresh < span`. `start_fetching_servers()` is single-flight, like the C# `listTask`: under `fetch_state` it either starts a resolve (`in_flight = true`), waits on the `watch` channel for the resolve already running, or returns immediately when the list is empty and the last resolve succeeded (the C# does not retry a completed empty task). A populated but stale list is force-refreshed; an empty list is resolved normally. The provider and loader futures are wrapped in `catch_unwind`, so a panicking provider is reported as a failed resolve instead of aborting the caller.

*Replacement.* `replace_list(endpoint_list, write_provider)` is the synchronous public form; it blocks while the provider is written, like the C# `ReplaceList`. Internal callers use the private `replace_list_internal(..., servers_time)`. Both de-duplicate the incoming records (a linear `Vec::contains` scan, so O(n²); the C# uses LINQ `Distinct()`), set `servers_last_refresh` to `servers_time` or `now`, clear the state and re-add every distinct record through `add_core`. When `write_provider` is set, the *distinct* list is written back to the provider.

*Protocol expansion.* `add_core` pushes one `ServerInfo` per individual protocol flag; `protocol_flags` splits a `ProtocolTypes` mask into `TCP`, `UDP`, `WEB_SOCKET` in that order — the same order as the C#'s `ProtocolTypesExtensions.GetFlags` iterator. (`ProtocolTypes::flags()` in `networking/steam3/protocol_type.rs` already provides this; `protocol_flags` is a local duplicate of it.) Note that `ProtocolTypes::has` is the C#'s `HasFlagsFast` semantics, i.e. **any** bit in common, not the stricter BCL `Enum.HasFlag`.

*Marking.* `try_mark(end_point: SocketAddr, protocol_types, quality)` finds the matching entries and sets or clears `last_bad_connection_time_utc`:

- `Good` matches entries whose `record.end_point()` equals the given address **and** whose protocol mask has any bit in common with `protocol_types`.
- `Bad` matches entries whose `record.get_host()` equals the host of the address (an ordinal comparison, like the C#), which poisons every endpoint of that host at once.

`mark_server_as_bad(end_point)` is `try_mark(…, ProtocolTypes::ALL, Bad)`; `mark_server_as_good(end_point, protocol_types)` is the `Good` variant.

*Candidate selection.* `best_candidate(servers, supported_protocol_types)` is the heart of the algorithm:

```
for each entry whose protocol has any bit in common with `supported_protocol_types`:
    key = entry.last_bad_connection_time_utc.unwrap_or(UNIX_EPOCH)
    keep the entry with the strictly smallest key
```

Only a **strictly** smaller key replaces the incumbent, so entries with the same key keep their original insertion order. This reproduces the C#'s `OrderBy(o => o.Server.LastBadConnectionTimeUtc.GetValueOrDefault()).ThenBy(o => o.Index)`: a server that has never failed sorts first (`DateTime.MinValue` in C#, `SystemTime::UNIX_EPOCH` in Rust — both are the minimum of their respective ranges, so the relative order is identical), a server that failed most recently sorts last, and equal scores are broken by the insertion index.

The returned record is **narrowed to the single protocol flag of the chosen entry**: `ServerRecord::new(server.record.endpoint().clone(), server.protocol)`. The C# does exactly the same (`new ServerRecord(result.Record.EndPoint, result.Protocol)`), so a server that advertises `Tcp|Udp` is returned as two separate candidates, one per flag. The full `EndPoint` (including the DNS host name) is cloned into the returned record so the TLS server name survives.

`get_next_server_candidate_async(protocols)` fetches first (`start_fetching_servers`) and then scores; a fetch failure is returned as `Err` (`Result<Option<ServerRecord>, SteamError>`), like the C# throw. `get_next_server_candidate(protocols)` is the synchronous variant; it calls `wait_for_servers_fetched()` first, so it fetches when needed, and returns the best candidate of the current list. `get_next_server_candidate_internal` is the private path that resets old bad marks and scores under one write lock. `get_all_end_points()` also fetches first and returns the distinct records in insertion order. `force_refresh_server_list()` takes `self: &Arc<Self>`, starts the refresh and returns `()` - it does not wait, like the C#.

*Fallbacks.* The two fallback servers are process-global `LazyLock<std::sync::RwLock<String>>` values: `DEFAULT_SERVER_WEBSOCKET = "cmp1-sea1.steamserver.net:443"` and `DEFAULT_SERVER_NETFILTER = "ext1-sea1.steamserver.net:27017"`. They are read/written through `default_server_websocket()` / `set_default_server_websocket()` and the netfilter equivalents; the lock is poison-tolerant (`unwrap_or_else(|e| e.into_inner())`).

**Notes and gotchas:**

- **The C# ↔ Rust object graph.** The C# `SmartCMServerList` holds a `SteamConfiguration`, and the configuration holds the server list — a reference cycle that the .NET garbage collector collects. Rust has no cycle collector, so the port depends on `Arc<SteamConfigurationState>` instead: the state holds the provider, the `AllowDirectoryFetch` flag and the Web API settings, and does not point back at the list. This is why `SmartCMServerList::new` takes the *state* and not the `SteamConfiguration`.
- **The blocking accessors match the C#.** `GetNextServerCandidate` and `GetAllEndPoints` both call `WaitForServersFetched()` first, and so do the Rust `get_next_server_candidate` and `get_all_end_points` (`wait_for_servers_fetched`). `block_on_future` always runs the fetch on a scoped thread with a runtime of its own, so the caller's runtime is never used; if a fetch is already in flight on a current-thread runtime the call panics with an explicit message instead of hanging. The `_async` variants remain and are what `CMClient::connect` uses.
- **`try_mark` semantics differ for DNS records.** The C# compares `System.Net.EndPoint` values, and a `DnsEndPoint` never equals an `IPEndPoint`; a DNS-backed server therefore cannot be marked *good* in the C# when the caller passes the resolved `IPEndPoint` of the live connection. The Rust compares the resolved `SocketAddr` (`ServerRecord::end_point()`), so a DNS record **is** matched. The port is therefore slightly more forgiving than the C# here.
- **`try_mark(Bad)` matches the host only**, like the C#: the comparison is ordinal, so a case-variant DNS name does not poison the record.
- **`replace_list` is synchronous.** The C# calls `UpdateServerListAsync(...).GetAwaiter().GetResult()` while holding `listLock`; the Rust public `replace_list` blocks on the provider write through `block_on_future`, while the internal async path (`replace_list_internal`) writes outside the state lock, so a slow provider does not block readers.
- **The resolve is single-flight and its outcome is cached**, like the C# `listTask`: an empty successful resolve is not retried until forced, a failed resolve is retried on the next call, and concurrent callers wait on the same `watch` generation.
- **The de-duplication in `replace_list` is O(n²).** The lists involved are small (tens of entries), so this has never mattered, but it is not what the C#'s `Distinct()` does.
- **The time spans are stored as `AtomicU64` millisecond counts.** Sub-millisecond values are truncated; the C#'s `TimeSpan` is exact.
- **No lock is held across `.await`.** The single-flight state is a `parking_lot::Mutex<FetchState>` held only for the flag/generation updates; waiters use a `tokio::sync::watch` channel, so a slow directory fetch never blocks another resolution's bookkeeping.
- **The `Debug` impl is hand-written** and only prints `allow_directory_fetch` and whether a directory loader is present; the server list itself is not dumped.
- **The directory request carries the Web API key** when one is configured: `SmartCMServerList::new` passes `Some(state.web_api_key.clone())` into the loader, and `build_params` in `web_api.rs` emits `key=…` unless the key is empty.

#### `crates/steamkit/src/steam/discovery/server_record.rs` — 290 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/ServerRecord.cs`
**Purpose:** Holds the endpoint of a CM server together with the protocol types that server supports, and provides the equality semantics the rest of the discovery module depends on (the list de-duplicates with `Distinct()`, which in C# is `ServerRecord.operator ==`).
**How it works:**

*`EndPoint`.* The Rust counterpart of `System.Net.EndPoint`:

```rust
pub enum EndPoint {
    Ip(SocketAddr),                 // System.Net.IPEndPoint
    Dns { host: String, port: u16, address: SocketAddr },  // System.Net.DnsEndPoint
}
```

`EndPoint::dns(host, port, address)` builds the DNS variant. `socket_addr()` returns the address to connect to (the literal address for `Ip`, the *resolved* address for `Dns`). `host()` returns the host string — the IP text for `Ip`, the DNS name for `Dns`. `port()` returns the port.

*Equality and hashing.* `PartialEq` requires the same variant: two `Ip` endpoints are equal when their `SocketAddr`s are equal; two `Dns` endpoints are equal when the ports are equal **and** the host names compare equal case-insensitively (`eq_ignore_ascii_case`), mirroring `System.Net.DnsEndPoint.Equals`; an `Ip` is never equal to a `Dns`. `Hash` mixes a discriminant byte (`0`/`1`) with the fields, lowercasing the DNS host, so the hash agrees with the equality. `Display` prints `address` for `Ip` and `host:port` for `Dns`. `From<SocketAddr>` yields `EndPoint::Ip`.

*`ServerRecord`.* `{ end_point: EndPoint, protocol_types: ProtocolTypes }`. `end_point()` returns the `SocketAddr` to connect to (the *resolved* address for a DNS record); `endpoint()` returns the full `&EndPoint` including the DNS host name; `is_dns()` reports whether the record is DNS-backed; `protocol_types()` returns the mask; `get_host()` and `get_port()` delegate to the endpoint. `PartialEq` compares the endpoint **and** the protocol types; `Hash` mixes both; `Display` prints `"{endpoint} ({protocols:?})"`.

*Factories.*

- `create_server(host, port, protocol_types)` is **async**. It first tries to parse `host` as an `IpAddr`; on success it produces an `EndPoint::Ip`. Otherwise it resolves the name with `tokio::net::lookup_host((host, port))`, takes the first address, and produces an `EndPoint::Dns` that keeps the host name, the port and the resolved address.
- `create_socket_server(end_point: SocketAddr)` — a TCP/UDP record over a literal address.
- `try_create_socket_server(address: &str) -> Option<Self>` — parses `"ip:port"`; a port of `0` is rejected. This is the Rust spelling of the C#'s `TryCreateSocketServer(string, out ServerRecord)`.
- `create_dns_socket_server(address)` and `create_websocket_server(address)` parse `"host:port"` (port `443` when no colon is present) and produce a TCP/UDP or a WebSocket record respectively. Both are async because they resolve.
- `create_server_from_dns` is the shared helper: it splits on the **first** `:`; a non-numeric port yields `SteamError::InvalidArgument("Port number must be a valid integer value.")`, the same message as the C#'s `ArgumentException`.

*Helpers.* `resolve_host(host, port)` performs the lookup and maps a failure to `SteamError::Connection("Failed to resolve {host}: {error}")`. `tcp_udp_protocol_types()` builds the `Tcp|Udp` mask from the raw flag values (`ProtocolTypes::TCP.raw() | ProtocolTypes::UDP.raw()`) because `ProtocolTypes`' `BitOr` implementation is not part of the fixed porting contract.

**Notes and gotchas:**

- **The Rust resolves the host name when the record is created; the C# does not.** The C# stores a lazy `System.Net.DnsEndPoint`, and the name is only resolved when a connection is made. `SUMMARY.md` §8 fixes `IConnection::connect` to take a `SocketAddr`, so the port resolves eagerly and keeps both the name and the address. Two consequences: the factories that accept a host name are `async`, and a record created for an unresolvable name fails immediately with `SteamError::Connection` instead of at connect time.
- **The DNS name is not redundant.** It is needed as the TLS server name of a WebSocket connection, which is why `EndPoint::Dns` stores `host` next to `address` and why `ServerRecord::endpoint()` (as opposed to `end_point()`) exists.
- **`get_port()` returns `u16`; the C# returns `int`.** The C#'s `GetPort()` extracts the port from an `EndPoint`; the Rust narrows it to the wire type.
- **The port split is on the first colon**, so an IPv6 literal (`"::1:27017"`) is mis-parsed. The C#'s `CreateServerFromDns` has the same `IndexOf(':')` behaviour, so this is a faithful port rather than a new bug — but do not feed IPv6 literals to these factories.
- **The port default is 443** when the address contains no colon, matching the C#'s `DefaultPort`.
- **`create_socket_server` and `try_create_socket_server` are the only synchronous factories**, because they never need a name lookup.
- **The C#'s `ServerRecord` constructor is `internal`;** the Rust's `ServerRecord::new` is public so that callers can build records for servers they discovered themselves.

#### `crates/steamkit/src/steam/discovery/basic_server_list.rs` — 78 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/BasicServerList.cs`
**Purpose:** The persistence envelope shared by the two file-backed providers. The C# class is a `protobuf-net` `[ProtoContract]` holding `[ProtoMember(1)] List<BasicServerListProto> Servers`; this file owns the equivalent root-list framing, while the per-record codec stays in `basic_server_list_proto.rs`.
**How it works:** `BasicServerList { servers: Vec<BasicServerListProto> }`, with `servers()`, `from_server_records(&[ServerRecord])`, `encode() -> Vec<u8>` and `decode(&[u8]) -> Result<Self, SteamError>`. `encode` writes one `0x0A varint(len) payload` record per entry and nothing at all for an empty list; `decode` reads records until the buffer is exhausted, skipping unknown fields, and rejects a truncated buffer, an over-long varint or an unknown wire type with `SteamError::Malformed`. The framing reuses the `write_tag` / `write_varint` / `Cursor` helpers that `basic_server_list_proto.rs` exposes to the crate.
**Notes and gotchas:**

- **The on-disk bytes are unchanged from the pre-4.0.0 layout.** The envelope was extracted from the record codec, not altered, so cache files written by the earlier port still load.
- **The envelope is not `prost`.** `protobuf-net`'s root-list framing is not what `prost` emits for a `Vec` field, so it is hand-written, like the record codec.
- **Both providers share this type**, which is what makes the two cache formats identical; `FileStorageServerListProvider` and `IsolatedStorageServerListProvider` differ only in where the bytes live.

#### `crates/steamkit/src/steam/discovery/basic_server_list_proto.rs` — 280 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/BasicServerListProto.cs`
**Purpose:** Defines the on-disk record format that the file-based and isolated-storage server list providers persist, and implements a hand-written protobuf codec for the individual record; the root-list envelope lives in `basic_server_list.rs`. The C# class is a `protobuf-net` `[ProtoContract]`; the Rust port cannot use `prost` here because the wire format must stay byte-compatible with the C#'s `protobuf-net` serialisation.
**How it works:**

*Payload.* `BasicServerListProto { address: String, port: u16, protocols: Option<ProtocolTypes> }` — the C#'s `Address` (string), `Port` (int) and `Protocols` (a `ProtocolTypes?` behind a property). `protocols()` returns `Tcp|Udp` when the value is absent, which is exactly what the C#'s property getter does (`protocolTypes ?? (ProtocolTypes.Tcp | ProtocolTypes.Udp)`). `from_server_record` fills it from a `ServerRecord` (`get_host()`, `get_port()`, `protocol_types()`); `to_server_record` converts back through `ServerRecord::create_server`, i.e. it resolves a DNS address.

*Wire format* (documented in the module header):

```
file    := item*
item    := 0x0A varint(len) payload      // field 1, length delimited
payload := field1 address | field2 port | field3 protocols
field1  := 0x0A varint(len) utf8-bytes  // string
field2  := 0x10 varint(port)            // int32
field3  := 0x18 varint(protocols)       // enum
```

The root list is the `protobuf-net` "repeated field 1" layout (`Serializer.DeserializeItems<…>(stream, PrefixStyle.Base128, 1)`), which `BasicServerList::encode` / `BasicServerList::decode` implement (see `basic_server_list.rs`). Values holding their default (empty address, port 0, protocols 0) are not written, matching `protobuf-net`.

*Codec.* `encode` writes the three optional fields; `decode` reads them back. A `WireType` enum models the six protobuf wire types; `write_tag` emits `field << 3 | wire`; `write_varint` emits base-128 varints. A minimal `Cursor` reader provides `read_varint` (with a 64-bit overflow guard), `read_bytes` (with a length-overflow guard) and `skip` for unknown fields.

**Notes and gotchas:**

- **This is the only protobuf codec in the port that is not `prost`.** Everything else uses generated `prost` types; the persisted server list cannot, because `protobuf-net`'s root-list layout is not what `prost` produces for a `Vec` field.
- **The port is written as a varint of `u16`.** `protobuf-net` writes the C# `int` — same wire type, same value for the range 0…65535.
- **Unknown fields are skipped, not rejected.** A file written by a newer C# build that adds a field 4 will still be read by the Rust port.
- **A malformed file is a hard error.** `BasicServerList::decode` returns `SteamError::Malformed` for an unknown wire type, a truncated buffer or an over-long varint. The C#'s `protobuf-net` throws a `ProtoException` in the same situations, and both providers surface it as an error from `fetch_server_list` — *except* that the providers only catch `IOException`, so a corrupt file propagates in both implementations.
- **An invalid UTF-8 address is rejected** with `SteamError::Malformed("Invalid UTF-8 in server address: …")`.

#### `crates/steamkit/src/steam/discovery/i_server_list_provider.rs` — 39 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/IServerListProvider.cs`
**Purpose:** The abstraction over "where the server list is cached". Implementations decide where the list lives (memory, a file, isolated storage) and when it was last refreshed.
**How it works:** `#[async_trait] pub trait IServerListProvider: Send + Sync` with three required methods:

- `last_server_list_refresh() -> Result<SystemTime, SteamError>` — the UTC time the list was last written. Providers that never stored a list report `SystemTime::UNIX_EPOCH`, the Rust counterpart of the C#'s `DateTime.MinValue` for the age comparisons in `SmartCMServerList`.
- `fetch_server_list() -> Result<Vec<ServerRecord>, SteamError>` — the known servers.
- `update_server_list(endpoints: Vec<ServerRecord>) -> Result<(), SteamError>` — persist a new list.

Three provided (defaulted) aliases exist: `load_servers` and `resolve_servers` both forward to `fetch_server_list`, and `save_servers` forwards to `update_server_list`. They are Rust conveniences with no C# counterpart.

**Notes and gotchas:**

- **Every method is `async`.** The C# interface is synchronous in shape — `LastServerListRefresh` is a property and `FetchServerListAsync`/`UpdateServerListAsync` return `Task`s — but the file and isolated-storage providers do blocking I/O, so the Rust port pushes that I/O onto tokio's blocking-aware `tokio::fs`.
- **The trait is object-safe and `Send + Sync`.** It is stored as `Arc<dyn IServerListProvider>` in the configuration state and in `SmartCMServerList`, so it must be usable from any thread.
- **The three aliases exist for API symmetry with the C# naming** (`LoadServers`/`SaveServers` in older SteamKit versions) and are not used anywhere inside the library.

#### `crates/steamkit/src/steam/discovery/file_storage_server_list_provider.rs` — 106 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/FileStorageServerListProvider.cs`
**Purpose:** Persists the server list in a protobuf-encoded file so that a restart of the application does not require another Steam Directory query.
**How it works:** `FileStorageServerListProvider { filename: PathBuf }`, built with `new(impl Into<PathBuf>)`; `filename()` exposes the path. `read_server_list()` reads the file with `tokio::fs::read`, decodes it with `BasicServerListProto::decode_list` and converts every item with `to_server_record()`. `write_server_list(endpoints)` encodes the records with `BasicServerListProto::encode_list` and writes them with `tokio::fs::write`. The `IServerListProvider` impl reports `last_server_list_refresh` as the file's modification time, falling back to `SystemTime::UNIX_EPOCH` when the file cannot be stat'ed.

**Notes and gotchas:**

- **I/O failures are swallowed on purpose.** A read error logs a warning and yields an empty list; a write error logs a warning and returns `Ok(())`. The C# catches `IOException` in both places and does the same. A *malformed* file, on the other hand, is a hard error in both implementations, because the deserialisation happens outside the `IOException` catch.
- **A record that cannot be resolved is skipped, not fatal.** `to_server_record()` resolves DNS names; a failure logs a warning and the entry is dropped, so a stale entry with a dead host name cannot break the whole list. The C#'s `CreateServer` never fails (it stores a lazy `DnsEndPoint`), so this failure mode is new in the port.
- **The C# truncates the file** with `fileStream.SetLength(fileStream.Position)` after serialising; `tokio::fs::write` truncates by default, so the on-disk result is the same.
- **The C# does the work on a thread pool** (`Task.Run`); the Rust uses `tokio::fs`, which dispatches to the blocking pool.
- **The Rust catches every I/O error, the C# only `IOException`.** Permission errors and the like are `IOException`s on both platforms, so the observable difference is minimal.

#### `crates/steamkit/src/steam/discovery/isolated_storage_server_list_provider.rs` — 142 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/IsolatedStorageServerListProvider.cs`
**Purpose:** Persists the server list in a per-user store, so that two applications using the library do not fight over the same cache file.
**How it works:** The struct is `{ filename: PathBuf }`; `new()` uses `default_filename()`, which resolves to `<temp dir>/steamkit/serverlist.protobuf` (`FILE_NAME = "serverlist.protobuf"`, `DIRECTORY_NAME = "steamkit"`, joined onto `std::env::temp_dir()`, falling back to `"."` when the temp dir is empty). `Default` is implemented and delegates to `new()`. `read_server_list` / `write_server_list` use exactly the same protobuf codec and the same error semantics as the file provider, except that the write path first creates the parent directory with `tokio::fs::create_dir_all`. The `IServerListProvider` impl reports the file's modification time as the refresh timestamp.

**Notes and gotchas:**

- **.NET `IsolatedStorage` has no Rust equivalent.** It is a Windows-only .NET Framework concept and is not available on .NET Core / .NET 5+ either. The port therefore keeps the *file format* identical but replaces the *storage location* with a per-user directory below the OS temp directory.
- **The isolation is weaker than the C#'s.** `IsolatedStorageFile.GetUserStoreForAssembly()` is per user *and* per assembly; `<temp>/steamkit/` is per user (on Windows the temp dir is per user, and on Unix it is per user through `TMPDIR`) but shared between all programs that pick the same directory name. Two different applications using this library will therefore share the cache.
- **The file content is interchangeable with the C#'s.** A file written by the Rust provider can be read by the C# provider and vice versa, because the protobuf byte layout is identical.
- **`create_dir_all` failure is non-fatal** — it logs a warning and returns `Ok(())`, so the subsequent write attempt (and its own error handling) decides the outcome.
- **The default path is computed per instance**, not cached in a static, so a changed `TMPDIR` between two constructions yields two different files.

#### `crates/steamkit/src/steam/discovery/memory_server_list_provider.rs` — 50 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/MemoryServerListProvider.cs`
**Purpose:** Keeps the server list in memory only. It is the default provider of a `SteamConfiguration`.
**How it works:** `MemoryServerListProvider { servers: tokio::sync::RwLock<Vec<ServerRecord>>, last_updated: tokio::sync::RwLock<Option<SystemTime>> }`. `last_server_list_refresh` returns the stored timestamp or `SystemTime::UNIX_EPOCH`; `fetch_server_list` returns a clone of the vector; `update_server_list` replaces the vector and stamps `last_updated` with `SystemTime::now()`. `new()` and `Default` produce an empty provider.

**Notes and gotchas:**

- **This is the default provider.** `SteamConfigurationState::default()` installs `MemoryServerListProvider::new()`, so a default configuration never persists the server list; every process start asks the Steam Directory again.
- **The clone in `fetch_server_list` is a full deep copy** of the vector (the `ServerRecord`s are `Clone`), so the caller cannot mutate the provider's state.
- **The two locks are separate**, so a reader can observe the new server list with the old refresh timestamp for a moment. The C# has the same shape (a list and a separate `DateTime` field).

#### `crates/steamkit/src/steam/discovery/null_server_list_provider.rs` — 40 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Discovery/NullServerListProvider.cs`
**Purpose:** A provider that never returns a server. It is for consumers that populate the server list themselves — for example an application that already knows its CM servers, or a test that wants the discovery module out of the way.
**How it works:** `NullServerListProvider` is a unit struct (`Copy`, `Default`) with a `new()` constructor. `last_server_list_refresh` always returns `SystemTime::UNIX_EPOCH` (the counterpart of the C#'s `DateTime.MinValue`), `fetch_server_list` returns an empty vector, and `update_server_list` is a no-op.

**Notes and gotchas:**

- **The Unix epoch is not neutral.** Because `SmartCMServerList::resolve_server_list` compares the provider's refresh time against `server_list_before_refresh_time_span`, an epoch timestamp is always "old", so the first resolution always attempts a forced refresh. With `allow_directory_fetch = false` that means the list stays empty and the fallback servers are never installed either — the caller must call `replace_list` itself.
- **The C#'s provider is a class with settable state;** the Rust's is a zero-sized unit struct, so it is free to clone and copy.

---

#### 5.2 `cdn/` — content servers, manifests and chunks

The CDN module downloads game content from Steam's Steampipe servers. It has four responsibilities: talking HTTP to a content server (`client.rs`), knowing what a content server is (`server.rs`), decrypting and decompressing a downloaded chunk (`depot_chunk.rs`), and optionally routing the traffic through a LAN cache (`client_lancache.rs`).

The C# spells the module `SteamKit2.CDN` and splits the client across two files (`Client.cs` and `ClientLancache.cs`) because C# has partial classes. Rust has no partial classes, so the port puts the Lancache members in their own file and attaches them with a second `impl Client` block — the same trick is used for `discovery`, where the C#'s `partial` classes are split the same way.

#### `crates/steamkit/src/steam/cdn/mod.rs` — 4 lines
**C# counterpart:** none — the C# has no `CDN.cs`; the namespace is implicit.
**Purpose:** Declares the four modules of the `cdn` module tree.
**How it works:** `pub mod client; pub mod client_lancache; pub mod depot_chunk; pub mod server;`
**Notes and gotchas:** `client_lancache` adds members to `Client` through a second `impl` block; it is not an independent type and cannot be used on its own.

#### `crates/steamkit/src/steam/cdn/server.rs` — 180 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/CDN/Server.cs`
**Purpose:** The description of a single Steampipe content server: where it lives, how to reach it, and the metadata (load, proxy flags, allowed app ids) that the client needs when it picks a server.
**How it works:**

`ConnectionProtocol { Http, Https }` is the Rust counterpart of the C#'s nested `Server.ConnectionProtocol` enum (`HTTP = 0`, `HTTPS = 1`); `Http` is the `#[default]`, matching the C#'s zero value.

`Server` carries the C#'s properties as `pub(crate)` fields with public getters:

| field | C# property | type |
|---|---|---|
| `protocol` | `Protocol` | `ConnectionProtocol` |
| `host` | `Host` | `Option<String>` |
| `v_host` | `VHost` | `Option<String>` |
| `port` | `Port` | `i32` |
| `server_type` | `Type` | `Option<String>` |
| `source_id` | `SourceID` | `i32` |
| `cell_id` | `CellID` | `u32` |
| `load` | `Load` | `i32` |
| `weighted_load` | `WeightedLoad` | `f32` |
| `num_entries` | `NumEntries` | `i32` |
| `steam_china_only` | `SteamChinaOnly` | `bool` |
| `use_as_proxy` | `UseAsProxy` | `bool` |
| `proxy_request_path_template` | `ProxyRequestPathTemplate` | `Option<String>` |
| `allowed_app_ids` | `AllowedAppIds` | `Vec<u32>` |
| `bypass_proxies_of_type` | `BypassProxiesOfType` | `Vec<String>` |

`from_socket_addr(end_point)` and `from_dns_end_point(host, port)` are the Rust spellings of the C#'s implicit conversions from `IPEndPoint` and `DnsEndPoint`: the protocol is `Https` when the port is 443 and `Http` otherwise, and `host` and `v_host` are both set to the same value. `From<SocketAddr> for Server` delegates to `from_socket_addr`. `Display` prints `"{host}:{port} ({type})"`, the same shape as the C#'s `ToString`.

**Notes and gotchas:**

- **`Type` is renamed to `server_type`** because `type` is a Rust keyword.
- **The fields are `pub(crate)`, not `pub`.** The C# exposes them with `internal set`; the Rust exposes read-only getters publicly and keeps the fields crate-internal, so only `ContentServerDirectoryService::convert_server_list` and the two constructors can populate them.
- **The port is an `i32`,** as in the C#, even though it is a TCP port. `build_command` interpolates it straight into the URL.
- **`host` and `v_host` are separate in the data model but identical in the two constructors.** The C#'s directory service is what sets them apart: `Host` is the machine to connect to, `VHost` is the name to put in the URL and the `Host` header.
- **The C#'s `Server` is `sealed` and mutable;** the Rust's is `Clone` and immutable from the outside.

#### `crates/steamkit/src/steam/cdn/depot_chunk.rs` — 174 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/CDN/DepotChunk.cs`
**Purpose:** Decrypts and decompresses a single depot chunk, and verifies the result against the chunk metadata that came with the depot manifest.
**How it works:**

*Wire format* (documented in the module header):

```
chunk       := iv[16] ciphertext[..]
iv          := AES-256-ECB(depotKey, iv)         // the IV is stored ECB encrypted
ciphertext  := AES-256-CBC(depotKey, iv, plaintext) with PKCS#7 padding
plaintext   := "VSZa" zstd-frame                 // Valve zstd container
             | "VZa"  lzma-frame                 // Valve LZMA container ("VZ" + version 'a')
             | "PK\x03\x04" zip                  // a plain PKzip archive
```

A chunk has no header of its own; its metadata (the SHA-1 chunk id, the Adler32 checksum of the decompressed data, the offset and the compressed/uncompressed lengths) lives in `types::depot_manifest::ChunkData`, which is produced by the types wave, not by this file.

*`DepotChunk::process(info: &ChunkData, data: &[u8], destination: &mut [u8], depot_key: &[u8]) -> Result<usize, SteamError>`*:

1. `destination.len()` must be at least `info.uncompressed_length`, otherwise `SteamError::InvalidArgument`.
2. `depot_key.len()` must be exactly 32 (`DEPOT_KEY_SIZE`), otherwise `SteamError::InvalidArgument("Tried to decrypt depot chunk with non 32 byte key!")`. The C# only asserts this in a `DebugLog.Assert`, i.e. in debug builds.
3. `data.len()` must be at least 16 (`AES_BLOCK_SIZE`), otherwise `SteamError::Malformed`.
4. The cipher is `aes::Aes256` with the depot key as the raw 256-bit key. **The IV is derived by decrypting the first 16 bytes of the input with AES in ECB mode**: `cipher.decrypt_block(&mut block)` over `data[..16]`. This is the "key derivation" step of the format — the depot key is used directly as the AES key, and the IV is stored encrypted under that same key.
5. The ciphertext is `data[16..]`. It must be non-empty and a multiple of 16 bytes, otherwise `SteamError::Malformed("The depot chunk has an invalid length …")`.
6. The ciphertext is copied into a `Vec` (`ciphertext.to_vec()`) and decrypted in place with `cbc::Decryptor::<aes::Aes256>::new_from_slices(depot_key, &iv)` and `decrypt_padded_mut::<Pkcs7>`. The number of plaintext bytes is `written`; `written < 16` yields `SteamError::Decompression("Not enough data in the decrypted depot chunk …")`.
7. The plaintext is dispatched on its magic: `VSZa` → `VZstdUtil::decompress(plaintext, destination, false)`, `VZa` → `VZipUtil::decompress(plaintext, destination, false)`, `PK\x03\x04` → `ZipUtil::decompress(&mut Cursor::new(plaintext), destination, false)`. Anything else is `SteamError::Decompression("Unexpected depot chunk compression (first four bytes are …)")`.
8. The written length must equal `info.uncompressed_length`, otherwise `SteamError::Decompression`.
9. The Adler32 of `destination[..written]` must equal `info.checksum`, otherwise `SteamError::Decompression("Processed data checksum is incorrect! …")`.
10. The number of bytes written is returned.

*`DepotChunk::adler_hash(input: &[u8]) -> u32`* implements Adler-32 with `BASE = 65521`, `s1` and `s2` both starting at **zero**. This matches the C#'s `Adler32.Calculate(0, input)`: .NET's implementation seeds the running sum `s1` with the low 16 bits of the seed, so a zero seed starts `s1` at 0 — which is *not* the RFC 1950 value of 1. Both implementations therefore agree with each other and differ from a textbook Adler-32; the checksum values in the manifest are computed the same way, so this is correct.

**Notes and gotchas:**

- **The buffer bug the test suite caught.** The first Rust implementation allocated a **zeroed** buffer and never copied the ciphertext into it, and it decrypted the IV out of the *zeroed* `iv` array instead of out of `data[..16]`. The file compiled and the API looked right, but it had never decrypted anything. The suite exposed it (§12 of this guide, defect 2). The fix was to decrypt the IV from `data[..AES_BLOCK_SIZE]` and to decrypt into `ciphertext.to_vec()` — a copy of the ciphertext — instead of a zeroed allocation. The same class of defect existed in `util/crypto_helper.rs::symmetric_decrypt` (defect 1) and is the reason the lesson "a port that only compiles is not a port that works" is recorded in §12 of this guide.
- **The C# rents its buffer from `ArrayPool`**, which may return a buffer *larger* than requested. The C# therefore must slice with `buffer.AsSpan(0, written)` when it inspects the plaintext, and it does. The Rust's buffer is exactly the ciphertext length, but the same discipline applies: the plaintext is taken as `&buffer[..written]`, never as the whole buffer. A depot chunk is PKCS#7 padded, so the plaintext is always shorter than the ciphertext.
- **The three compression containers are dispatched on the first four bytes**, and the magic is checked *after* decryption. A chunk encrypted with the wrong key therefore usually fails at the magic check (or at the PKCS#7 unpadding) rather than at the checksum.
- **The checksum verification uses `verifyChecksum: false` inside the decompressors**, exactly as the C# does. The Adler32 comparison in `process` is the authoritative integrity check; the per-container checksums are skipped because the depot chunk's own Adler32 already covers the data.
- **`DecryptCbc` vs `decrypt_padded_mut`.** The C#'s `aes.DecryptCbc(input, iv, buffer, PaddingMode.PKCS7)` returns the plaintext length and removes the padding in one step. The Rust's `decrypt_padded_mut::<Pkcs7>` does the same but returns a borrow into the buffer, which is why the plaintext length has to be captured before the borrow ends.
- **The key length check is a hard error in Rust and a debug-only assert in C#.** A 16-byte key would have been silently accepted by a release build of the C#.
- **`destination` must be at least `uncompressed_length`**, not `compressed_length`: with a depot key the output is the *decompressed* chunk. The callers in `cdn/client.rs` enforce this before the download starts.

#### `crates/steamkit/src/steam/cdn/client.rs` — 464 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/CDN/Client.cs` (the first half; the Lancache half is `ClientLancache.cs`)
**Purpose:** Downloads depot manifests and depot chunks from a Steampipe content server, honouring the CDN auth token, the proxy-server rewriting and the optional Lancache routing.
**How it works:**

*Type and construction.* `Client { http_client: reqwest::Client }`. `Client::new(http_client)` takes a ready-made client. `Client::from_steam_client(&SteamClient)` is the direct counterpart of the C# constructor `Client(SteamClient)`: it takes the configuration's `HttpClientFactory` and calls it with `HttpClientPurpose::CDN`. `Client::with_default_client()` builds a plain `reqwest::Client`. Note the contrast with the discovery module: **the CDN client invokes the HTTP client factory eagerly, at construction**, exactly like the C#.

*Timeouts.* `REQUEST_TIMEOUT` (10 s) and `RESPONSE_BODY_TIMEOUT` (60 s) are process-global `AtomicU64` counters holding milliseconds, with `request_timeout()` / `set_request_timeout()` and `response_body_timeout()` / `set_response_body_timeout()` accessors. The C# has the same two values as static settable `TimeSpan` properties.

*Constants.* `MANIFEST_VERSION = 5` — the manifest format version that goes into the URL.

*`download_manifest_async(depot_id, manifest_id, manifest_request_code, server, depot_key, proxy_server, cdn_auth_token)`*:

1. The command is `depot/{depot_id}/manifest/{manifest_id}/5`, with `/{manifest_request_code}` appended when the request code is greater than zero.
2. `build_command(server, command, cdn_auth_token, proxy_server)` turns it into a `reqwest::Url`.
3. `send_request` performs the request under a `tokio::time::timeout(request_timeout)`; a non-2xx status becomes `SteamError::Other("Response status code does not indicate success: {status}")`.
4. `read_body` reads the whole body under `tokio::time::timeout(response_body_timeout)`.
5. `extract_first_zip_entry(&body)` inflates the manifest: the manifest is a ZIP archive with exactly one entry.
6. `DepotManifest::deserialize(&manifest_data)` parses it.
7. When a depot key was supplied, `depot_manifest.decrypt_filenames(depot_key)` decrypts the file names.

*`download_depot_chunk_async(depot_id, chunk, server, destination, depot_key, proxy_server, cdn_auth_token)`*:

1. `chunk.chunk_id` must be present, otherwise `SteamError::InvalidArgument("Chunk must have a ChunkID.")`.
2. The destination is checked: with no depot key it must hold at least `chunk.compressed_length` bytes; with a depot key at least `chunk.uncompressed_length` bytes.
3. The URL is `depot/{depot_id}/chunk/{hex(chunk_id)}` (`hex::encode`, lower case).
4. The request is built through the Lancache path when `Client::use_lancache_server()` is true, otherwise through `build_command`.
5. The content length is taken from the response header when present. If the header is present and `chunk.compressed_length > 0`, the two must match, otherwise `SteamError::Malformed("Content-Length mismatch for depot chunk! …")`. Without a header the chunk's own `compressed_length` is used; if that is zero too, `SteamError::Other("Response does not have Content-Length and chunk.CompressedLength is not set.")`.
6. The body length must equal the content length, otherwise `SteamError::Malformed("Length mismatch after downloading depot chunk! …")`.
7. Without a depot key the bytes are copied verbatim into `destination` and the content length is returned. With a depot key the chunk is handed to `DepotChunk::process(chunk, &body[..content_length], destination, depot_key)`, which decrypts, decompresses and verifies the checksum.

*`build_command(server, command, query, proxy_server)`* builds the URL:

1. The scheme comes from `server.protocol` (`http`/`https`).
2. The host is `server.v_host`; a missing virtual host is `SteamError::InvalidArgument("The server does not have a virtual host.")`.
3. The port is `server.port`; the path is `/{command}` with any leading slashes of the command stripped.
4. If a proxy server is given **and** `proxy_server.use_as_proxy` is true **and** it has a `proxy_request_path_template`, the template is applied: `%host%` is replaced with the original virtual host and `%path%` with the path, and the scheme, host and port are taken from the proxy server instead. A proxy without a virtual host is `SteamError::InvalidArgument("The proxy server does not have a virtual host.")`.
5. A missing leading slash is added to the path, the URL is parsed, and a non-empty query (the CDN auth token) is appended with `set_query`.

*`extract_first_zip_entry(data)`* is a hand-written ZIP reader: `find_end_of_central_directory` scans backwards for the `0x06054b50` end-of-central-directory record (at most `0xFFFF + 22` bytes back); the central directory header (`0x02014b50`) yields the entry count, the compression method, the compressed and uncompressed sizes and the local header offset; the local file header (`0x04034b50`) yields the name and extra field lengths so the data offset can be computed. Compression method 0 (stored) returns the bytes as-is, method 8 (deflate) is inflated with `flate2::read::DeflateDecoder`; any other method is `SteamError::Decompression("Unsupported zip compression method {other}")`.

**Notes and gotchas:**

- **The Rust buffers the whole body; the C# streams it.** The C# sends with `HttpCompletionOption.ResponseHeadersRead` and copies the body into a rented buffer (manifest) or directly into the caller's destination buffer (chunk, when no depot key is used). The Rust reads the complete body into a `bytes::Bytes` first. For a depot chunk that means the compressed chunk is held in memory in addition to the destination buffer; for a manifest it means the whole archive is buffered. The C#'s chunk path avoids the extra copy when no depot key is given, the Rust does not.
- **Only "stored" and "deflate" ZIP entries are supported.** `System.IO.Compression.ZipArchive` handles more methods; Steam's manifests are deflate, so this is not a practical limitation, but a manifest compressed with another method fails with `SteamError::Decompression`.
- **The manifest's length is not validated.** The C# checks `ms.Position != contentLength` for the manifest response; the Rust only checks the length on the chunk path. A truncated manifest therefore surfaces as a ZIP parse error rather than a length mismatch.
- **The C# asserts that the manifest ZIP contains exactly one entry** (`DebugLog.Assert`, debug builds only) and then takes `entries[0]`. The Rust takes the first entry and only errors when the entry count is zero ("Expected the zip to contain only one file"). A multi-entry archive is not rejected.
- **The non-2xx error type differs.** The C# throws `SteamKitWebRequestException` (which carries the response); the Rust returns `SteamError::Other` with the status text.
- **The timeout covers only the request.** `send_request` wraps `request.send()`, `read_body` wraps `response.bytes()`. The two phases have separate timeouts, matching the C#'s `CancellationTokenSource` being re-armed with `ResponseBodyTimeout` after the headers arrive.
- **The `%path%` substitution uses the already normalised path.** The Rust substitutes the path *with* its single leading slash; the C# substitutes `$"/{uriBuilder.Path}"` and relies on `UriBuilder` for the slash. The resulting URLs are the same for a normal command.
- **The proxy is only honoured when it is marked `UseAsProxy` *and* has a path template**, exactly like the C#. A proxy server without a template is ignored and the original server is used.
- **The CDN auth token is passed as a raw query string**, not escaped — the C# does the same (`UriBuilder.Query = query`).

#### `crates/steamkit/src/steam/cdn/client_lancache.rs` — 121 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/CDN/ClientLancache.cs`
**Purpose:** Detects a Lancache instance on the local network and routes content downloads through it instead of to Steam's CDN. This is the Rust counterpart of the C#'s second `partial class Client` file.
**How it works:** A second `impl Client` block adds:

- `USE_LANCACHE_SERVER: AtomicBool` — the process-global switch, the counterpart of the C#'s `public static bool UseLancacheServer { get; private set; }`. `Client::use_lancache_server()` reads it.
- `detect_lancache_server_async()` — resolves `lancache.steamcontent.com` (port 80) with `tokio::net::lookup_host`. If **any** resolved address is a private address, the switch is enabled; otherwise it is disabled. The mechanism is the poisoned DNS entry that a Lancache installation maintains for that name.
- `is_private_address(&IpAddr)` — loopback is private; for IPv4 the ranges `10/8`, `172.16/12` and `192.168/16`; for IPv6 the unique-local range `fc00::/7`, link-local and loopback.
- `build_lancache_request(http_client, server, command, query)` — builds `http://lancache.steamcontent.com:80/{command}` (the query is appended when non-empty), sets the `Host` header to the **origin** server's host (so the cache knows which upstream to fill) and the `User-Agent` header to `Valve/Steam HTTP Client 1.0`, which Lancache requires in order to recognise Valve's CDN content.

**Notes and gotchas:**

- **The Rust's private-address check also covers IPv6 unique-local and link-local addresses**, matching the C#'s `(bytes[0] & 0xFE) == 0xFC || toTest.IsIPv6LinkLocal` plus the loopback test. The C# tests the loopback before it branches on the address family; the Rust does the same for IPv4 and handles the IPv6 loopback inside the IPv6 arm.
- **The switch is process-global and not reset automatically.** The C#'s `UseLancacheServer` is `private set` and only ever written by `DetectLancacheServerAsync`; the Rust's `AtomicBool` behaves the same. It defaults to `false`, so Lancache is opt-in until `detect_lancache_server_async()` succeeds.
- **The Rust's `lookup_host` already filters to IP addresses,** so the C#'s explicit `AddressFamily` filter is unnecessary. The C# resolves the name without a port; the Rust resolves `("lancache.steamcontent.com", 80)`, which is equivalent for a name lookup.
- **The Lancache path bypasses the proxy server entirely.** `build_lancache_request` takes neither a `proxy_server` nor a `Server` protocol, so a Lancache download always goes to `http://lancache.steamcontent.com:80`. This matches the C#.
- **The manifest download is never routed through the Lancache.** Only `download_depot_chunk_async` consults `use_lancache_server()`; `download_manifest_async` always goes to the real content server. This matches the C#, where `DownloadManifestAsync` has no Lancache branch.
- **`build_lancache_request` is `pub(crate)`**, so it is only reachable from inside the `cdn` module.

---

#### 5.3 `webapi/` — the Steam Web API client

The Web API module talks to `https://api.steampowered.com/`. The C# implementation is a `DynamicObject`: `api.ISteamDirectory.GetCMListForConnect(cellid: 1)` is resolved at run time by a `DynamicObject` override, which parses the member name, flattens list arguments and picks the HTTP method from a reserved `method` argument. Rust has no dynamic dispatch, so the port replaces it with explicit methods and an explicit argument list.

There are two "interfaces" in the C# sense, and they are what the task means by the `INTERFACE`/`METHOD` string interface: the **interface name** (`ISteamDirectory`, `IContentServerDirectoryService`) and the **method name** (`GetCMListForConnect`, `GetServersForSteamPipe`), plus a version number. The URL is always `{base}/{INTERFACE}/{METHOD}/v{version}/`.

> There is no `ApiCall` or `ApiResult` type in the C# `WebAPI.cs`, and none in the Rust port. The closest equivalents are the call methods on `AsyncInterface` (`call`, `call_protobuf`, `call_protobuf_with_request`) and the `WebAPIResponse<T>` result envelope. Do not look for those names in the tree.

#### `crates/steamkit/src/steam/webapi/mod.rs` — 5 lines
**C# counterpart:** none — the C# has no `WebAPI.cs` module file; the namespace is implicit.
**Purpose:** Declares the five modules of the `webapi` module tree.
**How it works:** `pub mod content_server_directory_service; pub mod steam_configuration_web_api_extensions; pub mod steam_directory; pub mod web_api; pub mod web_api_request_exception;`
**Notes and gotchas:** `steam_directory.rs` and `content_server_directory_service.rs` are the two "service" helpers; `web_api.rs` is the transport; `steam_configuration_web_api_extensions.rs` is the glue to `SteamConfiguration`; `web_api_request_exception.rs` is the error type.

#### `crates/steamkit/src/steam/webapi/web_api.rs` — 661 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/WebAPI/WebAPI.cs`
**Purpose:** The transport for the Steam Web API: builds the request URL, encodes the arguments, performs the request under a timeout, and decodes the response as VDF (`KeyValue`) or protobuf.
**How it works:**

*Constants.* `DEFAULT_BASE_ADDRESS = "https://api.steampowered.com/"` and `DEFAULT_TIMEOUT = Duration::from_secs(100)`. `WebAPI::default_base_address()` parses the constant (a parse failure is impossible, so it is `expect`ed) and `WebAPI::default_timeout()` returns the timeout.

*Escaping.* `uri_unreserved()` builds the character set that `System.Uri.EscapeDataString` leaves untouched — `A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~` — out of `percent_encoding`'s `NON_ALPHANUMERIC`, and caches it in a `OnceLock<AsciiSet>` because `percent_encoding` ships no constant for exactly that set. `escape_data_string(value)` applies it. `url_encode_bytes(bytes)` is the counterpart of `HttpUtility.UrlEncode` for a `byte[]`: every byte becomes `%XX` with upper-case hex.

*Argument model.*

```rust
pub enum WebApiValue { Text(String), Bytes(Vec<u8>), Null }
pub struct WebApiArgument { pub name: String, pub value: WebApiValue }
pub type WebApiArgs = Vec<WebApiArgument>;
```

`WebApiValue` preserves the three value shapes of the C#'s `Dictionary<string, object?>`: `Null` for a `null` value, `Bytes` for a `byte[]` and `Text` for everything else (the C# renders it with `ToString()` and then escapes it). `From` impls exist for `String`, `&str`, `Vec<u8>`, `i32`, `u32`, `i64`, `u64` and `bool`; the integer and boolean conversions all produce `Text`. `web_api_args([…])` is a small array-based builder.

*Argument ordering is deterministic.* The C# uses a `Dictionary<string, object?>`, whose enumeration order is an implementation detail; the Rust uses an ordered `Vec`, so the query string is stable. This matters for the `key`/`format` prefix and for reproducible requests.

*Result envelope.* `WebAPIResponse<TResponse> { result: EResult, body: TResponse }` — the C#'s `WebAPI.WebAPIResponse<TResponse>` with `Result` and `Body`.

*`WebAPI` (a unit struct with associated functions).* `get_interface(iface, api_key)`, `get_interface_with_base_address(base_address, iface, api_key)`, `get_async_interface(iface, api_key)` and `get_async_interface_with_base_address(...)` all build an interface over a default HTTP client created by `create_default_http_client()` — a `reqwest::Client` with a 100 s timeout.

*`Interface`.* A thin facade over `Arc<AsyncInterface>`: `iface()`, `timeout()` / `set_timeout()`, `async_interface()`, and the two call methods. Its methods are `async fn` — the C#'s `Interface.Call` blocks on the underlying task (`callTask.Wait(Timeout)`), and blocking a tokio worker thread is not allowed.

*`AsyncInterface`.* `{ http_client: reqwest::Client, base_address: reqwest::Url, iface: String, api_key: String, timeout_millis: AtomicU64 }`. `iface()`, `api_key()`, `base_address()`, `timeout()` / `set_timeout()` (the timeout is a per-interface `AtomicU64` of milliseconds, defaulting to 100 s).

*The call methods.*

- `call(func, version, args)` — a GET in the `vdf` format; the response is parsed with `KeyValue::read_as_text` and a parse failure becomes `SteamError::Malformed("An internal error occurred when attempting to parse the response from the WebAPI server. This can indicate a change in the VDF format. …")`.
- `call_with_method(method, func, version, args)` — the same, with an explicit HTTP method.
- `call_protobuf<T>(method, func, version, args)` — a request in the `protobuf_raw` format; the body is decoded with `T::decode` (`T: prost::Message + Default`). The C#'s `CallProtobufAsync<T>` is the counterpart.
- `call_protobuf_with_request<TResponse, TRequest>(method, func, request, version, extra_args)` — the request message is serialised with `prost`, base64-encoded and sent as the `input_protobuf_encoded` argument. Extra arguments are appended, skipping any whose name is already present. The result code is read from the **`x-eresult` response header** and defaults to `EResult::Invalid` when the header is missing or unparsable. The C#'s `CallProtobufAsync<TResponse, TRequest>` is the counterpart.

*`call_internal(method, func, version, args, expected_format)`* is the shared core:

1. If the caller supplied a `format` argument it must equal the expected format, otherwise `SteamError::InvalidArgument("Unsupported 'format' value '…'. Format must either be '…' or omitted.")`. (The C# has the same check but hard-codes `"vdf"` in the comparison, so the C# rejects a `format=protobuf_raw` argument even on the protobuf path; the Rust compares against the format the call actually wants.)
2. `build_params(api_key, expected_format, format_provided, args)` produces the parameter string: `key=<escaped key>` (only when the key is non-empty and the caller did not supply a `key` argument), then `format=<expected>` (only when the caller did not supply a `format`), then `&<escaped name>=<value>` for every argument. A `Null` value produces a bare `&name=`; a `Bytes` value is percent-encoded byte by byte; a `Text` value is escaped.
3. The URL is `base_address.join("{iface}/{func}/v{version}/")`. A GET puts the parameters in the query string; any other method puts them in an `application/x-www-form-urlencoded` body.
4. The request is performed under `tokio::time::timeout(self.timeout(), request.send())`. A timeout is `SteamError::Timeout`; a non-2xx status becomes `WebAPIRequestException::status(code, reason)` converted into `SteamError::Connection`.

**Notes and gotchas:**

- **No dynamic dispatch.** The C#'s `TryInvokeMember` parses the member name with `(?<name>[a-zA-Z]+)(?<version>[0-9]*)` (so `GetCMListForConnect001` means version 1), flattens `IEnumerable` arguments into `name[0]`, `name[1]`, …, and treats an argument literally named `method` as the HTTP method. The Rust caller must pass the method name, the version and the argument list explicitly. This is the single largest API-shape difference in the port.
- **The per-request timeout covers the headers, not the body.** `call_internal` wraps `request.send()` in the timeout, but the body is read afterwards with `response.bytes()` outside it. The C#'s `HttpClient.Timeout` covers the whole operation including the body. A slow body therefore hangs until the `reqwest` client's own timeout (if any) fires, not until the interface timeout. The CDN client does not have this problem — it has a separate body timeout.
- **The C#'s `Interface.Call` throws `TimeoutException`**; the Rust returns `SteamError::Timeout`.
- **The base address cannot be set on the client.** The C# assigns `client.BaseAddress = config.WebAPIBaseAddress` on the `HttpClient`; a `reqwest::Client` is immutable once built, so the base address travels on the interface and the URL is built with `Url::join`.
- **The `format` check differs from the C#** (see step 1 above): the Rust compares against the format the call actually needs, the C# always compares against `"vdf"`.
- **`WebAPIResponse` is only produced by the protobuf-with-request overload**, exactly like the C#.
- **There is no `Dispose`.** The C#'s `Interface`/`AsyncInterface` implement `IDisposable` and dispose the `HttpClient`. A `reqwest::Client` is `Clone` and shares its connection pool between clones, so the Rust has nothing to dispose; dropping the last clone releases the pool.
- **`WebAPIResponse::result` defaults to `EResult::Invalid`**, not to `OK`, when the `x-eresult` header is absent. The C# has the same default (`var eresult = EResult.Invalid;`).

#### `crates/steamkit/src/steam/webapi/steam_configuration_web_api_extensions.rs` — 71 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/WebAPI/SteamConfigurationWebAPIExtensions.cs`
**Purpose:** Builds a Web API interface from an existing `SteamConfiguration`, using the configuration's HTTP client factory and Web API settings.
**How it works:** Rust has no extension methods, so the operations live on a trait `SteamConfigurationWebApiExtensions` that is implemented for `SteamConfiguration`:

```rust
pub trait SteamConfigurationWebApiExtensions {
    fn get_web_api_interface(&self, iface: &str) -> Interface;
    fn get_async_web_api_interface(&self, iface: &str) -> AsyncInterface;
}
```

The free functions `get_web_api_interface(config, iface)` and `get_async_web_api_interface(config, iface)` do the same without the trait, and the two service helpers (`steam_directory.rs`, `content_server_directory_service.rs`) use the free functions. `get_http_client_for_web_api(config)` is the single point that touches the factory:

```rust
pub(crate) fn get_http_client_for_web_api(config: &SteamConfiguration) -> reqwest::Client {
    let factory = config.http_client_factory();
    factory(HttpClientPurpose::WebAPI)
}
```

**The `HttpClientPurpose` enum** itself is *not* declared in this file; it lives in `steam/steam_client/configuration/steam_configuration.rs`:

```rust
pub enum HttpClientPurpose { CMWebSocket, WebAPI, CDN }
```

It is the reason a user can give the library three differently configured HTTP clients (a WebSocket client with a proxy, a Web API client with a long timeout, a CDN client with a large body limit). `HttpClientFactory` is `Arc<dyn Fn(HttpClientPurpose) -> reqwest::Client + Send + Sync>`.

**Why the Web API client is created lazily.** The C#'s `GetHttpClientForWebAPI` calls `config.HttpClientFactory(HttpClientPurpose.WebAPI)` *when the interface is requested*, and then mutates the returned client: `client.BaseAddress = config.WebAPIBaseAddress; client.Timeout = WebAPI.DefaultTimeout;`. A `reqwest::Client` cannot be reconfigured after it is built, so the port cannot do either of those things. The base address is therefore carried by the `Interface`/`AsyncInterface` (which is where the URL is built) and the timeout is applied per request by the interface itself. The timeout the *factory* configured on the client is deliberately **not** overridden — the doc comment records this as an explicit deviation.

The practical consequences are:

- **Building a `SteamConfiguration` never calls the factory.** The factory runs when an interface is requested, and again for every subsequent request, because `get_http_client_for_web_api` calls it each time.
- **A fresh `reqwest::Client` per request means a fresh connection pool per request.** `reqwest::Client` is expensive to build and holds a connection pool, so requesting an interface once and reusing it is strongly preferred over calling `get_web_api_interface` in a loop. (This is the same reason the discovery module caches its client in a `OnceLock`.)
- **The factory's own timeout still applies to the whole request**, in addition to the interface's per-request timeout.

**Notes and gotchas:**

- **The C#'s extension methods are `static` members of a static class**; the Rust offers both a trait (for method-call syntax) and free functions (for call sites that do not want to import the trait). The two are interchangeable.
- **`HttpClientPurpose` has three variants in Rust and three in C#** (`CMWebSocket`, `WebAPI`, `CDN`); the C# enum is in `SteamKit2/SteamClient/Configuration/SteamConfiguration.cs`.
- **The default factory** (`default_http_client_factory()`) ignores the purpose and builds a `reqwest::Client` with the user agent `SteamKit/{version}`.
- **`get_http_client_for_web_api` is `pub(crate)`**; the public surface is the trait and the two free functions.

#### `crates/steamkit/src/steam/webapi/web_api_request_exception.rs` — 54 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/WebAPI/WebAPIRequestException.cs`
**Purpose:** The error type raised when a Web API request fails with a non-success status code.
**How it works:** A `thiserror` enum:

```rust
pub enum WebAPIRequestException {
    Status { status: u16, reason: String },   // "Response status code does not indicate success: {status} ({reason})."
    Other  { message: String },
}
```

`status(status, reason)` and `other(message)` are the constructors. `From<WebAPIRequestException> for SteamError` maps a `Status` failure to `SteamError::Connection(message)` and an `Other` failure to `SteamError::Other(message)`. The `Status` variant's `Display` output is the same sentence the C# puts into its exception message.

**Notes and gotchas:**

- **The C# derives from `SteamKitWebRequestException`**, which lives in `SteamKit2.Util` (`Util/SteamKitWebRequestException.cs`). The `util` section of the port had not been written when this file was created, so the Rust declares a standalone `thiserror` type that converts into `SteamError`. The file's header records that the integrator can lift it into `crate::util::steam_kit_web_request_exception` once that module exists.
- **The Rust type is an enum, not a class with subclasses.** A C# caller that catches `WebAPIRequestException` and inspects `Response` cannot do the equivalent here: the Rust variant carries only the status code and the reason phrase, not the response.
- **Only the status-code path is exercised by the library.** `Other` exists for completeness and is what a caller would build for a non-status failure.

#### `crates/steamkit/src/steam/webapi/steam_directory.rs` — 118 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/WebAPI/SteamDirectory.cs`
**Purpose:** Queries the Steam Directory Web API (`ISteamDirectory/GetCMListForConnect`) and turns the response into `ServerRecord`s. It is the source of the CM server list that the discovery module installs.
**How it works:** `SteamDirectory` is a unit struct with four entry points — `load(configuration)`, `load_with_cancellation(configuration, cancellation)`, `load_with_max_num_servers(configuration, max_num_servers, cancellation)` and `load_with_interface(directory, cell_id, max_num_servers, cancellation)`. The first three delegate to the private `load_core`, which obtains the `ISteamDirectory` interface with `get_async_web_api_interface(configuration, "ISteamDirectory")` and then calls `load_with_interface`.

`load_with_interface` is the part that does not depend on the configuration, which is what lets the discovery module reuse it with a client of its own:

1. The arguments are `cellid` (the cell id) and, when `max_num_servers` is given, `maxcount`.
2. The cancellation token is checked before the call.
3. `directory.call_with_method(reqwest::Method::GET, "GetCMListForConnect", 1, &args)` performs the request.
4. The cancellation token is checked again.
5. The response's `success` flag is checked with `as_boolean(false)`; a failure is `SteamError::Other("Steam Web API returned failure - {message}")` where the message comes from the response's `message` field (or `"unknown"`).
6. For every child of the response's `serverlist` node, the `endpoint` is read. The child's `type` decides the record: `"websockets"` → `ServerRecord::create_websocket_server(endpoint)`, `"netfilter"` → `ServerRecord::create_dns_socket_server(endpoint)`, anything else is skipped.

**Notes and gotchas:**

- **The "china" server type is not handled.** The Steam Directory also returns a `"china"` type that uses WebSockets. The C# carries a `// TODO: There's also "china" type which uses websocket` comment and skips it; the Rust does the same and says so in a comment.
- **`create_websocket_server` and `create_dns_socket_server` are async in Rust** because they resolve the host name (see `server_record.rs`). The C# stores a lazy `DnsEndPoint` and never resolves. This means a directory response with a broken host name fails the whole query in the Rust, whereas the C# would only fail later, at connect time.
- **The response is requested in the `vdf` format**, because `call_with_method` uses the `vdf` expected format. The C# does the same.
- **The cancellation token is a `tokio_util::sync::CancellationToken`**, the port's replacement for the C#'s `CancellationToken`. It is only checked at two points; it does not abort an in-flight HTTP request.
- **`max_num_servers` is an `i32` here** and maps onto the `maxcount` argument; the C# uses an `int` as well.
- **`load_with_interface` is public** precisely so that `discovery/smart_cm_server_list.rs` can query the directory with a client it manages itself.

#### `crates/steamkit/src/steam/webapi/content_server_directory_service.rs` — 141 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/WebAPI/ContentServerDirectoryService.cs`
**Purpose:** Queries the content-server directory (`IContentServerDirectoryService/GetServersForSteamPipe`) and turns the protobuf response into `cdn::Server` records.
**How it works:** `ContentServerDirectoryService` is a unit struct with `load`, `load_with_cancellation`, `load_with_cell_id` and `load_with_cell_id_and_max_num_servers`; all of them delegate to the private `load_core(configuration, cell_id, max_num_servers, cancellation)`, which:

1. Obtains the `IContentServerDirectoryService` interface with `get_async_web_api_interface(configuration, "IContentServerDirectoryService")`.
2. Builds a `CContentServerDirectoryGetServersForSteamPipeRequest` with `cell_id` (the caller's value, or `configuration.cell_id()`) and, when given, `max_servers`.
3. Checks the cancellation token.
4. Calls `call_protobuf_with_request::<CContentServerDirectoryGetServersForSteamPipeResponse, CContentServerDirectoryGetServersForSteamPipeRequest>(GET, "GetServersForSteamPipe", &request, 1, &[])`.
5. Checks the cancellation token again.
6. Converts the response with `convert_server_list`.

`convert_server_list(response)` maps every entry of `response.servers` onto a `cdn::Server`:

| proto field | `Server` field | note |
|---|---|---|
| `https_support` | `protocol` | `"mandatory"` → `Https` (port 443), otherwise `Http` (port 80) |
| `host` | `host` | |
| `vhost` | `v_host` | |
| — | `port` | 443 for HTTPS, 80 for HTTP |
| `type` | `server_type` | |
| `source_id` | `source_id` | |
| `cell_id` | `cell_id` | `as u32` |
| `load` | `load` | |
| `weighted_load` | `weighted_load` | |
| `num_entries_in_client_list` | `num_entries` | |
| `steam_china_only` | `steam_china_only` | |
| `use_as_proxy` | `use_as_proxy` | |
| `proxy_request_path_template` | `proxy_request_path_template` | |
| `allowed_app_ids` | `allowed_app_ids` | |
| `bypass_proxies_of_type` | `bypass_proxies_of_type` | |

**Notes and gotchas:**

- **The port is derived from the protocol, not from the proto.** The proto message has no port field; the C# and the Rust both use 443 for HTTPS and 80 for HTTP.
- **`cell_id` is truncated from `i32` to `u32`**, exactly like the C#'s `(uint)child.cell_id`.
- **Optional proto fields default to zero / false** through `unwrap_or_default()`, where the C#'s protobuf-net leaves them at their default value.
- **This is the only place in the port that populates a `cdn::Server` with a `v_host` that differs from its `host`.** The two constructors in `cdn/server.rs` always set them to the same value.
- **The `x-eresult` of the response is discarded.** `call_protobuf_with_request` returns a `WebAPIResponse` whose `result` is the `x-eresult` header value, but `load_core` only looks at `response.body`. A non-`OK` result with a decodable body is therefore not reported as an error.
- **The C#'s `LoadAsync` overloads take the cell id as `uint`;** the Rust's `load_with_cell_id` takes a `u32` as well.

---

#### 5.4 `authentication/` — QR and password login

The authentication module implements the Steam login. It is the only path in the library that can log on to a real Steam account **without a password on the command line**: the QR flow asks Steam for a challenge URL, the user scans it with the Steam mobile app, and the session is confirmed by polling. It is therefore the recommended way to test a live login, and the port ships it as `crates/steamkit-samples/src/bin/001_authentication_with_qr_code.rs`.

> Two names in the brief do not exist in the code. There is no `PollPendingAuthSession` in either implementation — the method is **`PollAuthSessionStatus`** (C#) and **`poll_auth_session_status`** (Rust), and it is a *unified messaging service method* of the `Authentication` service, not a Steam client method. There is also no type called `finalizer`; the role the brief gives it is played by `AuthSession::polling_wait_for_result`, which turns a live session into an `AuthPollResult` carrying the refresh token and the access token, and — when a separate audience token is needed — by `SteamAuthentication::generate_access_token_for_app`, which exchanges a refresh token for a fresh access token.

#### The flow, end to end

**QR (recommended for a live test).**

1. `SteamClient::connect(None)` connects to a CM server. The client is now *connected but not logged on*; that is the state the authentication service is reached in.
2. `client.authentication().begin_auth_session_via_qr(&AuthSessionDetails::new())` calls the `Authentication.BeginAuthSessionViaQR` service method. The response carries `client_id`, `request_id`, `allowed_confirmations`, `interval` and `challenge_url`.
3. The challenge URL is a `https://steam.com/qr/…` link that encodes the client id. The caller renders it as a QR code; the user scans it with the Steam mobile app and confirms the sign-in.
4. Steam refreshes the challenge URL periodically. The Rust port surfaces this through `QrAuthSession::challenge_url_changed()`, a `tokio::sync::broadcast::Receiver<String>`; the C#'s `QrAuthSession.ChallengeURLChanged` is an `Action?` with no payload.
5. `session.polling_wait_for_result(cancellation)` polls `Authentication.PollAuthSessionStatus` every `interval` seconds until the response carries a non-empty `refresh_token`. The returned `AuthPollResult` holds `account_name`, `refresh_token`, `access_token` and an optional `new_guard_data`.
6. `SteamUser::log_on(&LogOnDetails { username, access_token: Some(refresh_token), .. })` logs on with the token. Note the field mapping: the **refresh** token is what goes into `LogOnDetails::access_token` — this is what the C# sample does as well.

**Password.**

1. `client.authentication().begin_auth_session_via_credentials(&details)` requires `details.username` and `details.password`.
2. The password is encrypted: `Authentication.GetPasswordRSAPublicKey` returns the account's RSA public key (hex-encoded modulus and exponent) and a timestamp; the password is encrypted with **RSA PKCS#1 v1.5** and base64-encoded.
3. `Authentication.BeginAuthSessionViaCredentials` is called with the account name, the persistence flag, the website id, the guard data, the encrypted password, the encryption timestamp and the device details.
4. The response carries the SteamID and the same `client_id` / `request_id` / `allowed_confirmations` / `interval` quartet.
5. If the account is behind Steam Guard, `allowed_confirmations` tells the session which confirmation type to use. `polling_wait_for_result` drives the `IAuthenticator` for the code-based types and polls for the mobile-confirmation type.
6. The result is again an `AuthPollResult`.

**How the C# model maps onto the Rust traits.**

| C# | Rust |
|---|---|
| `AuthSession` (base class) | `AuthSession` (struct) + the `SessionKind` enum |
| `CredentialsAuthSession : AuthSession` | `CredentialsAuthSession` (owns an `AuthSession`) |
| `QrAuthSession : AuthSession` | `QrAuthSession` (owns an `AuthSession` + a shared `QrAuthSessionState`) |
| `protected virtual HandlePollAuthSessionStatusResponse` | `AuthSession::handle_poll_auth_session_status_response`, which inspects `SessionKind` |
| `IAuthenticator` (interface) | `IAuthenticator` (`#[async_trait]` trait) |
| `UserConsoleAuthenticator` (class) | `UserConsoleAuthenticator` (struct implementing the trait) |
| `AuthSessionDetails` (class with settable properties) | `AuthSessionDetails` (struct with public fields) |
| `AuthPollResult` / `AccessTokenGenerateResult` | same names, plain structs with `from_response` constructors |
| `AuthenticationException` | `AuthenticationException` (`thiserror` enum) + `SteamError::Authentication` |

The C# uses class inheritance to model "a session is either a credentials session or a QR session". Rust has no inheritance, so the kind-specific state lives in the `SessionKind` enum and the two concrete session types *own* an `AuthSession` rather than deriving from it. The consequence for a caller: the two session types expose the same accessor set (`client_id`, `request_id`, `polling_interval`, `allowed_confirmations`, `authenticator`, `polling_wait_for_result`, `poll_auth_session_status`) but there is no common trait or base type to hold them, so a generic function over "any session" has to be written twice or over an enum.

#### `crates/steamkit/src/steam/authentication/mod.rs` — 10 lines
**C# counterpart:** none — the C# has no `Authentication.cs` module file; the namespace is implicit.
**Purpose:** Declares the ten modules of the `authentication` module tree.
**How it works:** `access_token_generate_result`, `auth_poll_result`, `auth_session`, `auth_session_details`, `authentication_exception`, `credentials_auth_session`, `i_authenticator`, `qr_auth_session`, `steam_authentication`, `user_console_authenticator`.
**Notes and gotchas:** `steam_authentication.rs` is the entry point; it is reached through `SteamClient::authentication()`.

#### `crates/steamkit/src/steam/authentication/steam_authentication.rs` — 509 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/SteamAuthentication.cs`
**Purpose:** The handler that talks to Steam's `Authentication` unified messaging service: it starts sessions, polls them, sends Steam Guard codes and generates access tokens.
**How it works:**

*Service plumbing.* The service name is `"Authentication"` and the service version is `1`; the target job name of a request is `"{AUTHENTICATION_SERVICE}.{method}#{AUTHENTICATION_SERVICE_VERSION}"`, e.g. `Authentication.BeginAuthSessionViaQR#1`.

*The generated service client is replaced by a hand-rolled one.* The C# obtains `Internal.Authentication` from the unified-messaging handler (`unifiedMessages.CreateService<Internal.Authentication>()`), a `protobuf-net` generated service client that does the request/response matching for it. `prost` does not generate service clients, so the port sends the service method itself:

- `SteamAuthentication::call_service::<Req, Resp>(method, request)` allocates a job id with `client.get_next_job_id()`, registers a `tokio::sync::oneshot::Sender<RawServiceResponse>` for that job id, builds a `ClientMsgProtobuf<Req>` with `with_payload_reserve(e_msg, 64)`, sets the source job id and `header.proto.target_job_name`, writes the body and sends it. It then awaits the oneshot and decodes the response with `Resp::decode`.
- The message type is `EMsg::ServiceMethodCallFromClient` when the client is already logged on and `EMsg::ServiceMethodCallFromClientNonAuthed` otherwise — the login flow runs in the latter state.
- `RawServiceResponse { result: EResult, body: Vec<u8> }` carries the raw answer; `PendingRequests` is `Arc<parking_lot::Mutex<HashMap<JobID, oneshot::Sender<RawServiceResponse>>>>` with `insert`, `remove` and `complete`.
- `AuthenticationServiceHandler` implements `ClientMsgHandler`; it accepts `EMsg::ServiceMethodResponse` protobuf messages, reads `header.proto.jobid_target` as the job id and `header.proto.eresult` as the result, extracts the body from `proto_packet.body_offset()` onwards and completes the pending request. `SteamAuthentication::new(client)` registers this handler with the client.
- If `client.send(&msg)` fails, the pending entry is removed and the error is reported as `AuthenticationException::other("Failed to send the authentication request: …")`. If the caller gives up, the oneshot send fails and is ignored.

*Service methods.*

- `get_password_rsa_public_key(account_name)` → `GetPasswordRSAPublicKey`. A non-`OK` result is `AuthenticationException::failed("Failed to get password public key", result)`.
- `generate_access_token_for_app(steam_id, refresh_token, allow_renewal)` → `GenerateAccessTokenForApp`. `renewal_type` is set to `KETokenRenewalTypeAllow` when `allow_renewal` is true. The result is an `AccessTokenGenerateResult`.
- `begin_auth_session_via_qr(details)` → `BeginAuthSessionViaQR`. The client must be connected (`SteamError::InvalidArgument("The SteamClient instance must be connected.")` otherwise). The request carries `website_id` and the device details. The response is turned into a `QrAuthSessionState` (holding `challenge_url`) and an `AuthSession` with `SessionKind::Qr`.
- `begin_auth_session_via_credentials(details)` → `BeginAuthSessionViaCredentials`. It requires a non-empty username **and** password (`SteamError::InvalidArgument("BeginAuthSessionViaCredentials requires a username and password to be set in 'details'.")`) and a connected client. It then fetches the RSA public key, encrypts the password, and sends the request with the persistence flag (`KESessionPersistencePersistent` / `KESessionPersistenceEphemeral`), the website id, the guard data, the base64 password, the encryption timestamp and the device details. The response's `steamid` becomes the session's `SteamID`.
- `poll_auth_session_status(client_id, request_id)` → `PollAuthSessionStatus`. The result can be `Expired`, `FileNotFound` or `Fail`; anything but `OK` is `AuthenticationException::failed("Failed to poll status", result)`.
- `send_steam_guard_code(client_id, steam_id, code, code_type)` → `UpdateAuthSessionWithSteamGuardCode`. `EResult::DuplicateRequest` is **tolerated** — it happens when the user already accepted the prompt in the mobile app and the code is then sent anyway; the authentication succeeds on the next poll. Every other non-`OK` result is an error.

*Helpers.* `device_details(details)` builds the `CAuthenticationDeviceDetails` (the friendly name, the platform type, the OS type). `encrypt_password(public_key, password)` hex-decodes the modulus and the exponent, imports them as `rsa::BoxedUint` values into an `rsa::PublicKey` and encrypts with `pkcs1v15_encrypt` — the counterpart of the C#'s `RSAEncryptionPadding.Pkcs1`.

**Notes and gotchas:**

- **The service client is hand-rolled, which is the biggest structural difference in this module.** Everything the C#'s generated client does — job id allocation, request/response matching, the `EMsg` choice, the `target_job_name` — is explicit here.
- **`get_password_rsa_public_key` is `pub(crate)` in Rust**; the C#'s method is `private`. The credentials flow uses it internally.
- **The C#'s `BeginAuthSessionViaCredentialsAsync` checks the credentials before the connection state,** and the Rust does the same. The QR flow in the C# checks the connection state first, and so does the Rust.
- **`EResult::DuplicateRequest` is not an error** for `send_steam_guard_code`. This is a deliberate, documented carve-out.
- **The oneshot is a one-shot rendezvous.** A response that arrives after the caller was cancelled is dropped silently (`let _ = sender.send(response);`), exactly like a `TaskCompletionSource` whose result nobody awaits.
- **`SteamAuthentication` holds a `Weak<SteamClient>`**, so the handler never keeps the client alive. Every operation upgrades the weak reference and reports `AuthenticationException::other("The SteamClient instance has been dropped.")` when the client is gone.
- **The handler is registered in `SteamAuthentication::new`.** If the client is already dropped at that point, no handler is registered and every service call fails with the "dropped" error.

#### `crates/steamkit/src/steam/authentication/auth_session.rs` — 413 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/AuthSession.cs`
**Purpose:** The state machine of a login: it holds the session identifiers, drives the Steam Guard confirmation, and polls Steam until the session produces tokens.
**How it works:**

*`SessionKind`.*

```rust
pub(crate) enum SessionKind {
    Credentials { steam_id: SteamID },
    Qr { state: Arc<QrAuthSessionState> },
}
```

This enum replaces the C#'s class hierarchy. The QR variant carries a shared, mutable state object so that the base session can publish a refreshed challenge URL to the `QrAuthSession` the caller holds.

*`AuthSession`.* `{ client: Weak<SteamClient>, authenticator: Option<Arc<dyn IAuthenticator>>, client_id: AtomicU64, request_id: Vec<u8>, allowed_confirmations: Vec<CAuthenticationAllowedConfirmation>, polling_interval: Duration, kind: SessionKind }`. The constructor sorts the confirmations and converts the C#'s `float` polling interval into a `Duration` with `Duration::from_secs_f64`. `client_id` is an `AtomicU64` because the server can change it mid-session.

*Confirmation ordering.* `sort_confirmations` orders the allowed confirmations by the C#'s preference list: `None`, `DeviceConfirmation`, `DeviceCode`, `EmailCode`, `EmailConfirmation`, `MachineToken`, `Unknown`; anything unrecognised sorts last. `confirmation_type` maps the proto's integer `confirmation_type` onto `EAuthSessionGuardType` (1 → None, 2 → EmailCode, 3 → DeviceCode, 4 → DeviceConfirmation, 5 → EmailConfirmation, 6 → MachineToken, 7 → LegacyMachineAuth, anything else → Unknown).

*`polling_wait_for_result(cancellation)`.*

1. The first allowed confirmation is the preferred one. An empty list, or a preferred confirmation whose type is `Unknown`, is `SteamError::InvalidArgument("There are no allowed confirmations")`.
2. If there is an authenticator and the preferred type is `DeviceConfirmation`, the authenticator is asked whether it wants to simply poll for the mobile confirmation (`accept_device_confirmation()`). If it answers `false` and there is no second confirmation type, the call fails with "AcceptDeviceConfirmation returned false which indicates a fallback to another confirmation type, but there are no other confirmation types available."; otherwise the second confirmation becomes the preferred one.
3. The guard type then decides the path:
   - `None` — no Steam Guard; nothing to do.
   - `EmailCode` or `DeviceCode` — an authenticator is required, otherwise `SteamError::InvalidArgument("This account requires an authenticator for login, but none was provided in AuthSessionDetails.")`. The expected "the code was wrong" result is `EResult::InvalidLoginAuthCode` for the email type and `EResult::TwoFactorCodeMismatch` for the device type. The loop asks the authenticator for a code (passing the `previous_code_was_incorrect` flag), sends it with `send_steam_guard_code`, and repeats as long as the failure is the expected "wrong code" result. An empty code is `SteamError::InvalidArgument("No code was provided by the authenticator.")`.
   - `DeviceConfirmation` — the session enters the polling loop: the user confirms in the mobile app.
   - Anything else — `SteamError::InvalidArgument("Unsupported confirmation type …")`.
4. Without a poll loop, the session is polled once and the result is required; a `None` result becomes `SteamError::Authentication(AuthenticationException::failed("Authentication failed", EResult::Fail))`.
5. With a poll loop, the session sleeps for `polling_interval` (in a `tokio::select!` against the cancellation token) and polls again, until a result arrives or the token is cancelled.

*Polling.* `poll_auth_session_status()` returns `Result<Option<AuthPollResult>, SteamError>`; `poll_auth_session_status_raw()` is the same call but keeps the `AuthenticationException` (and therefore the `EResult`) intact. The raw variant upgrades the `Weak<SteamClient>`, calls `SteamAuthentication::poll_auth_session_status(client_id, request_id)`, feeds the response through `handle_poll_auth_session_status_response` and returns `Some(AuthPollResult)` when the response's `refresh_token` is non-empty, `None` otherwise.

*Handling the response.* `handle_poll_auth_session_status_response(response)` is the counterpart of the C#'s `protected virtual` method. It updates `client_id` from `new_client_id` (when it is not zero) and, for a QR session, publishes `new_challenge_url` into the shared `QrAuthSessionState` when the URL is non-empty.

*`send_steam_guard_code(code, code_type)`* is only valid for a credentials session; on a QR session it is `AuthenticationException::other("Got {code_type:?} confirmation type in a session that is not CredentialsAuthSession.")`.

**Notes and gotchas:**

- **The C# rejects a code-type confirmation in a non-credentials session up front** (`if (this is not CredentialsAuthSession …) throw new InvalidOperationException(…)`). The Rust checks the session kind inside `send_steam_guard_code`, so the error surfaces one step later — after the authenticator has already been asked for a code.
- **The polling interval comes from the server** (`response.interval`, a float in seconds). The C# uses `TimeSpan.FromSeconds((double)pollingInterval)`; the Rust uses `Duration::from_secs_f64`.
- **`client_id` is mutable and shared.** The server can hand out a new client id in a poll response; the session updates it, so a caller that cached `client_id()` early may see a stale value. `client_id()` reads the atomic each time.
- **The `DeviceConfirmation` path never calls the authenticator for a code.** It only asks `accept_device_confirmation()` once, before the loop, to decide whether the mobile prompt is acceptable.
- **The cancellation token is honoured at the code boundaries and between polls**, not inside an in-flight request.
- **The C#'s `PollingWaitForResultAsync` returns a non-nullable `AuthPollResult` and throws on failure;** the Rust returns `Result<AuthPollResult, SteamError>` with the same semantics.
- **`allowed_confirmations` is sorted once, at construction,** exactly like the C#'s `SortConfirmations` in the constructor. The order is therefore stable for the lifetime of the session.

#### `crates/steamkit/src/steam/authentication/auth_session_details.rs` — 99 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/AuthSessionDetails.cs`
**Purpose:** The parameters of a login request: who is logging on, from what device, with what persistence and which Steam Guard data.
**How it works:** A `Clone` struct with public fields:

| field | C# property | default |
|---|---|---|
| `username: Option<String>` | `Username` | `None` |
| `password: Option<String>` | `Password` | `None` |
| `device_friendly_name: Option<String>` | `DeviceFriendlyName` | `"{machine} (SteamKit2)"` |
| `platform_type: EAuthTokenPlatformType` | `PlatformType` | `KEAuthTokenPlatformTypeSteamClient` |
| `client_os_type: EOSType` | `ClientOSType` | `Utils::get_os_type()` |
| `is_persistent_session: bool` | `IsPersistentSession` | `false` |
| `website_id: Option<String>` | `WebsiteID` | `Some("Client")` |
| `guard_data: Option<String>` | `GuardData` | `None` |
| `authenticator: Option<Arc<dyn IAuthenticator>>` | `Authenticator` | `None` |

`Default` (and `new()`) install the C#'s defaults. `machine_name()` reads `COMPUTERNAME` and falls back to `HOSTNAME`, then to `"unknown"` — the counterpart of `System.Environment.MachineName`.

**Notes and gotchas:**

- **The password is redacted in the `Debug` output** (`<redacted>`), which the C# does not do. The authenticator is printed as the type name only.
- **`username` and `password` are only needed for the credentials flow.** The QR flow ignores them; `AuthSessionDetails::new()` is enough to start a QR session.
- **`website_id` is a free-form string.** The known values are "Unknown", "Client", "Mobile", "Website", "Store", "Community", "Partner" and "SteamStats"; the default is "Client".
- **`guard_data` carries the `AuthPollResult::new_guard_data` of a previous login**, so that an email-based Steam Guard account does not need a new email on every login. The doc comment points at `auth_poll_result::AuthPollResult::new_guard_data`.
- **The struct is `Clone`, so the authenticator is shared** (`Arc<dyn IAuthenticator>`) between the details and the session.
- **The C#'s properties are settable;** the Rust's fields are public and mutable, so the same style of code works.

#### `crates/steamkit/src/steam/authentication/qr_auth_session.rs` — 124 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/QrAuthSession.cs`
**Purpose:** The QR flavour of an authentication session: it owns the challenge URL and publishes it when Steam refreshes it.
**How it works:** `QrAuthSessionState { challenge_url: parking_lot::RwLock<String>, changed: tokio::sync::broadcast::Sender<String> }` is the shared, mutable part. It is created with a capacity-16 broadcast channel; `set_challenge_url` stores the new URL and sends it on the channel (a send with no subscribers is not an error); `challenge_url()` reads it; `subscribe()` returns a new receiver.

`QrAuthSession { session: AuthSession, state: Arc<QrAuthSessionState> }` owns both the base session and the shared state. `challenge_url()` returns the current URL; `challenge_url_changed()` returns a `broadcast::Receiver<String>`; `authenticator()`, `client_id()`, `request_id()`, `polling_interval()` and `allowed_confirmations()` forward to the base session; `polling_wait_for_result(cancellation)` and `poll_auth_session_status()` forward as well.

**Notes and gotchas:**

- **The C#'s `Action? ChallengeURLChanged` is a single callback with no payload**; it is invoked from the poll thread. The Rust's broadcast channel delivers the *new URL* to every subscriber, which is a superset of the C#'s behaviour and removes the "only one listener" limitation.
- **The channel has a capacity of 16.** A subscriber that falls more than 16 refreshes behind receives `RecvError::Lagged` and loses messages. The C#'s callback has no equivalent failure mode.
- **The challenge URL is only refreshed by a poll.** The `AuthSession::handle_poll_auth_session_status_response` hook pushes `new_challenge_url` into this state, so the URL changes at the polling cadence, not spontaneously.
- **The state is shared through an `Arc`**, which is how the base `AuthSession` (which knows only the `SessionKind`) can reach the `QrAuthSession`'s state. This is the Rust replacement for the C#'s `protected virtual` override.
- **The C#'s `ChallengeURL` has an `internal` setter;** the Rust's is written only through `QrAuthSessionState::set_challenge_url`, which is `pub(crate)`.

#### `crates/steamkit/src/steam/authentication/credentials_auth_session.rs` — 90 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/CredentialsAuthSession.cs`
**Purpose:** The password flavour of an authentication session. It adds the SteamID of the account that is logging on and the ability to send a Steam Guard code.
**How it works:** `CredentialsAuthSession { session: AuthSession }`. `steam_id()` returns the session's SteamID (the C# documents that it is only set when the credentials were correct). `send_steam_guard_code(code, code_type)` forwards to the base session, which refuses it for a non-credentials session. `authenticator()`, `client_id()`, `request_id()`, `polling_interval()`, `allowed_confirmations()`, `polling_wait_for_result(cancellation)` and `poll_auth_session_status()` forward to the base session.

**Notes and gotchas:**

- **The SteamID is only meaningful after a successful `BeginAuthSessionViaCredentials`.** The C# has the same caveat in its documentation.
- **There is no common base type between `CredentialsAuthSession` and `QrAuthSession`.** Code that wants to handle "any session" must match on both types or introduce its own enum.
- **`send_steam_guard_code` takes the code type explicitly**, because the C#'s caller passes `preferredConfirmation.confirmation_type`. The Rust's `polling_wait_for_result` does the same.

#### `crates/steamkit/src/steam/authentication/i_authenticator.rs` — 29 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/IAuthenticator.cs`
**Purpose:** The hook that supplies Steam Guard codes to a login.
**How it works:** `#[async_trait] pub trait IAuthenticator: Send + Sync` with three methods:

- `get_device_code(previous_code_was_incorrect: bool) -> Result<String, SteamError>` — called when the account uses the authenticator app.
- `get_email_code(email: &str, previous_code_was_incorrect: bool) -> Result<String, SteamError>` — called when the account uses Steam Guard email authentication. `email` is the address the guard mail was sent to (it comes from the confirmation's `associated_message`).
- `accept_device_confirmation() -> Result<bool, SteamError>` — called when the account has the Steam mobile app and accepts notification prompts. Returning `false` means "fall back to entering a code instead".

**Notes and gotchas:**

- **The trait is `Send + Sync`** because it is stored as `Arc<dyn IAuthenticator>` inside `AuthSessionDetails` and `AuthSession`, which live inside a `SteamClient` that is used from several tasks.
- **The `previous_code_was_incorrect` flag is the retry signal.** The C#'s implementation of `UserConsoleAuthenticator` uses it to print a different prompt; a custom authenticator can use it to avoid asking the user twice for the same code.
- **`accept_device_confirmation` returning `false` is only meaningful when `DeviceConfirmation` is the preferred confirmation type.** `polling_wait_for_result` only consults it in that case.

#### `crates/steamkit/src/steam/authentication/user_console_authenticator.rs` — 122 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/UserConsoleAuthenticator.cs`
**Purpose:** The default `IAuthenticator`: it prompts the user on the console for the Steam Guard code.
**How it works:** `UserConsoleAuthenticator { reader: Mutex<BufReader<Stdin>> }` (a `tokio::sync::Mutex`, so the read is serialised). `read_line()` reads one line and trims it; end of input yields `None`. `write_prompt(prompt)` writes to **stderr** and flushes. `get_device_code` prints "STEAM GUARD! Please enter your 2-factor auth code from your authenticator app: " and loops until a non-empty line is entered; `get_email_code` prints "STEAM GUARD! Please enter the auth code sent to the email at {email}: " and behaves the same way. Both print an extra "The previous 2-factor auth code you have provided is incorrect." line when `previous_code_was_incorrect` is set. `accept_device_confirmation` prints "STEAM GUARD! Use the Steam Mobile App to confirm your sign in..." and returns `true`.

**Notes and gotchas:**

- **The read is asynchronous.** The C# uses the blocking `Console.ReadLine`; the Rust uses `tokio::io::stdin`, so a tokio worker thread is never blocked while the user types. This is the port's standard treatment of blocking console I/O.
- **The prompts go to stderr, not stdout,** so that a program that prints structured data on stdout is not corrupted by the prompt. The C#'s implementation writes to the console without distinguishing the streams.
- **End of input yields an empty string**, not an error: `read_line` returns `None` and the method returns `Ok(String::new())`. The session then fails with "No code was provided by the authenticator." The C#'s `Console.ReadLine` returns `null` at EOF, which the C#'s session also treats as an empty code.
- **The loop skips empty lines** and asks again.
- **The `Mutex` is held only for the duration of one `read_line`.** Two concurrent calls to the authenticator would interleave their prompts, but the `AuthSession` never does that.

#### `crates/steamkit/src/steam/authentication/auth_poll_result.rs` — 33 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/AuthPollResult.cs`
**Purpose:** The outcome of a successful authentication: the tokens that let the client log on to Steam.
**How it works:** `AuthPollResult { account_name: String, refresh_token: String, access_token: String, new_guard_data: Option<String> }` with `from_response(&CAuthenticationPollAuthSessionStatusResponse)`, which unwraps the response's optional fields with `unwrap_or_default()`. The refresh token is the one to hand to `LogOnDetails::access_token`; the access token is subordinate to it.

**Notes and gotchas:**

- **`new_guard_data` is an `Option<String>` in Rust** and a plain string in the C#. It holds a remembered machine id, normally produced by an email-based Steam Guard login; feeding it back into `AuthSessionDetails::guard_data` avoids a new email on the next login. It should be stored per account.
- **The struct is `Default`**, so a default-constructed result has empty tokens. `poll_auth_session_status` only returns `Some` when the response's `refresh_token` is non-empty, so an empty `refresh_token` in a hand-built result would be a mistake.

#### `crates/steamkit/src/steam/authentication/access_token_generate_result.rs` — 24 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/AccessTokenGenerateResult.cs`
**Purpose:** The result of exchanging a refresh token for a fresh access token.
**How it works:** `AccessTokenGenerateResult { refresh_token: String, access_token: String }` with `from_response(&CAuthenticationAccessTokenGenerateForAppResponse)`. It is produced by `SteamAuthentication::generate_access_token_for_app(steam_id, refresh_token, allow_renewal)`.

**Notes and gotchas:**

- **The refresh token may legitimately be empty** in the response; the doc comment says so. The access token is the one to use with the Web API; the refresh token is the one to store.
- **`allow_renewal` sets `renewal_type = KETokenRenewalTypeAllow`**, which lets Steam renew the token later. Without it the generated token is a one-shot.

#### `crates/steamkit/src/steam/authentication/authentication_exception.rs` — 56 lines
**C# counterpart:** `SteamKit2/SteamKit2/Steam/Authentication/AuthenticationException.cs`
**Purpose:** The error type of the authentication subsystem, carrying the Steam result code when there is one.
**How it works:** A `thiserror` enum:

```rust
pub enum AuthenticationException {
    Failed { message: String, result: EResult },   // "{message} with result {result:?}."
    Other  { message: String },
}
```

`failed(message, result)` and `other(message)` are the constructors. `result()` returns `Some(EResult)` for a `Failed` error and `None` for an `Other` one. `From<AuthenticationException> for SteamError` maps everything to `SteamError::Authentication(value.to_string())`.

**Notes and gotchas:**

- **`result()` is what the retry loop inspects.** `polling_wait_for_result` compares the error's result against `InvalidLoginAuthCode` / `TwoFactorCodeMismatch` to decide whether to ask for another code; the conversion into `SteamError` throws the result code away, so the check has to happen before the conversion. That is why the module has both `poll_auth_session_status` (which converts) and `poll_auth_session_status_raw` (which does not).
- **The C#'s exception carries the result code in a `Result` property** and is thrown; the Rust's is a value that is returned.
- **Every authentication failure ends up as `SteamError::Authentication`,** so a caller that only inspects the top-level error type loses the `EResult`. The `Display` text keeps it, though.

---

## 6. `crates/steamkit-tools/` — the dev binaries


### 6.1 Overview


`crates/steamkit-tools` is the port's **build-time tooling crate**. It is the Rust counterpart of the
`Resources/` directory of the C# SteamKit2 tree, and it holds **four binaries**:

| Binary | C# counterpart | What it does |
|---|---|---|
| `steamkit-lang-gen` | `Resources/SteamLanguageParser` (`Program.cs`, `Parser/*`, `Generator/*`) | Parses Valve's `.steamd` enum/struct definition files and generates the Rust SteamLanguage module (`crates/steamkit/src/base/generated/steam_language.rs`). |
| `steamkit-proto-gen` | `Resources/ProtobufGen` | Regenerates the `prost` protobuf code from Valve's `.proto` files. |
| `steamkit-proto-dump` | `Resources/ProtobufDumper` | Extracts the protobuf definitions that are embedded in Steam's binaries and writes them back out as `.proto` source. |
| `steamkit-nethook-analyzer` | `Resources/NetHookAnalyzer2` (a WPF GUI) | Command-line port of the NetHook2 dump analyzer: lists and decodes the capture files that the NetHook2 sniffer writes. |

The crate is a workspace member (`Cargo.toml` → `members = [..., "crates/steamkit-tools", ...]`) and is
built with the rest of the workspace, but none of its code is linked into `crates/steamkit`; it is a
*tool* crate that only depends on `steamkit` and `steamkit-protos` to decode the data it inspects.

Two of the four binaries are **not literal ports**:

* `steamkit-proto-gen` / `steamkit-proto-dump` are reimplementations. The C# tools used protobuf-net's
  own `.proto` parser and code generator; the Rust ones drive the same vendored `protoc` that
  `crates/steamkit-protos/build.rs` uses, so the descriptors are byte-for-byte the ones the generated
  `prost` types are built from. `steamkit-proto-dump` additionally gained a `--protos <dir>` mode
  (see [06.4](#064-steamkit-proto-dump)) that the C# tool did not have.
* `steamkit-nethook-analyzer` is a CLI, not a GUI. The WPF parts of `NetHookAnalyzer2` (`MainForm.cs`,
  `TreeNodeObjectExplorer.cs`, the `ListView*` helpers) have no counterpart; the decoding logic does.

#### Module map

```
crates/steamkit-tools/
├─ Cargo.toml                     no build.rs
├─ verify_steam_language.py       enum-parity gate (C# vs generated Rust)
├─ lang/                          the 7 Valve .steamd definition files
├─ tools/gen_proto_registry.py    generator for src/nethook/proto_registry.rs
└─ src/
   ├─ lib.rs                      crate root; re-exports the four tool families
   ├─ main.rs                     steamkit-lang-gen
   ├─ tokenizer.rs                .steamd lexer
   ├─ analyzer.rs                 .steamd parser / symbol locator
   ├─ generator.rs                .steamd → Rust emitter
   ├─ error.rs                    LangError
   ├─ proto_error.rs              ProtoError
   ├─ proto_gen/                  steamkit-proto-gen
   │  ├─ mod.rs
   │  ├─ proto_set.rs             protoc driver + descriptor set
   │  └─ code_generator.rs        prost_build wrapper + Steam service trait generator
   ├─ proto_dumper/               steamkit-proto-dump
   │  ├─ mod.rs
   │  ├─ wire.rs                  minimal protobuf wire reader
   │  ├─ raw_options.rs           raw view of a descriptor (keeps unknown fields)
   │  ├─ executable_scanner.rs    finds descriptors inside a binary
   │  ├─ protobuf_collector.rs    parses candidate descriptors
   │  ├─ protobuf_dumper.rs       descriptor → .proto source
   │  └─ util.rs                  literal formatting helpers
   ├─ nethook/                    steamkit-nethook-analyzer
   │  ├─ mod.rs
   │  ├─ error.rs
   │  ├─ item.rs                  capture-file naming contract
   │  ├─ dump.rs                  a directory of capture files
   │  ├─ message_reading.rs       header/body/payload reader
   │  ├─ message_type_finder.rs   body type resolution
   │  ├─ message_type_overrides.rs
   │  ├─ proto_registry.rs        generated lookup tables (10 621 lines)
   │  ├─ serializable_registry.rs
   │  ├─ emsg_extensions.rs        game coordinator message names
   │  ├─ well_known_app_ids.rs
   │  ├─ protobuf_field_reader.rs  generic protobuf fallback reader
   │  ├─ body.rs                  decoded body model
   │  ├─ render.rs                body → tree of nodes
   │  ├─ tree.rs                  tree builder
   │  └─ specializations/         seven "extra object" decoders
   └─ bin/
      ├─ steamkit-proto-gen.rs
      ├─ steamkit-proto-dump.rs
      └─ steamkit-nethook-analyzer.rs
```

---

#### 06.2 `steamkit-lang-gen` — the SteamLanguage (`.steamd`) pipeline

#### The `.steamd` definition language

A `.steamd` file is Valve's own description of the Steam wire protocol's enums and message structs.
The language is tiny and fully covered by the grammar in `src/analyzer.rs`:

```
#import "other.steamd"                       preprocessor directive, pulls in another file

enum NAME { ... }                             plain enum
enum NAME<TYPE> { ... }                       enum with an explicit underlying type
enum NAME flags { ... }                       enum whose members are bit flags

class NAME { ... }                            message struct
class NAME<Enum::Member> { ... }             message struct bound to a message id
class NAME expects Parent { ... }             'expects' is parsed and dropped
class NAME removed { ... }                    the whole struct is not emitted
```

Inside an `enum` or `class` body a **member** is one of:

```
NAME;                         bare member (enum: next value; class: field of unknown type)
TYPE NAME;                    typed member
TYPE NAME = value;            typed member with a default
TYPE<opt> NAME = value;       typed member with a marshal option, e.g. `byte<20> loginKey;`
```

`value` is a `|`-separated list of terms. A term is a decimal/hexadecimal integer literal
(`0xBAADF00D`, `-700`), the name of a previously declared member of the *same* enum, or — through the
global literal table built by the generator — the name of a member of *another* enum that has a single
literal value. After the terminating `;` a member may carry a lifecycle marker:

```
NAME = 5; obsolete                kept, emitted as #[deprecated]
NAME = 6; obsolete "use X"       kept, emitted as #[deprecated = "use X"]
NAME = 7; removed                 dropped entirely
NAME = 8; removed "gone"         dropped entirely (the reason is consumed but not emitted)
```

The **marshal flags** that appear in Valve's files are:

| Flag | Occurrences | Meaning |
|---|---|---|
| `const` | 31 | The member is a compile-time constant, not a field. |
| `steamidmarshal` | 32 | The field is a `CSteamID`; Rust type `u64`. |
| `gameidmarshal` | 4 | The field is a `CGameID`; Rust type `u64`. |
| `boolmarshal` | 5 | The field is a `bool` stored as one byte. |
| `proto` | 2 | The field is a nested protobuf message; Rust type `Vec<u8>`. |
| `protomask` | 1 | The field carries a masked `EMsg` (the `MsgHdrProtoBuf.msg`). |
| `protomaskgc` | 1 | The field carries a masked GC `EMsg` (the `MsgGCHdrProtoBuf.msg`). |

Type names map to Rust primitives as `byte`→`u8`, `short`→`i16`, `ushort`→`u16`, `int`→`i32`,
`uint`→`u32`, `long`→`i64`, `ulong`→`u64`, `char`→`char`, `bool`→`bool`; any other type name is a
locally declared enum or class and becomes a generated Rust type.

#### The `.steamd` to Rust pipeline

```
lang/steammsg.steamd
   │  tokenize()      src/tokenizer.rs
   ▼
Vec<Token>
   │  analyze()       src/analyzer.rs      (recurses into #import)
   ▼
Root { children: Vec<Node> }
   │  Generator::generate()   src/generator.rs
   ▼
crates/steamkit/src/base/generated/steam_language.rs
```

The entry file is `steammsg.steamd`; it `#import`s `header.steamd`, which in turn imports
`emsg.steamd`, `eresult.steamd`, `enums.steamd` and `netheader.steamd`. `gamecoordinator.steamd` is
imported by `steammsg.steamd` directly. The imported files' nodes are **spliced into the importing
root after the importing file's own nodes have been parsed**, exactly like the C# `TokenAnalyzer`
recursion — the ordering of the generated enums therefore follows the import order, not the file order.

#### The alias mechanism (C# duplicate enum values → Rust associated `const`s)

C# permits two enum members to share a value; Rust rejects it with `error[E0081]: duplicate
discriminants`. Valve's `.steamd` files rely on this: e.g. in `emsg.steamd`

```
BaseGeneral = 100;
GenericReply = 100;
```

The generator therefore tracks, per enum, the set of values already used (`seen_values`). The **first**
member that carries a value becomes a real variant; every later member with the same value becomes an
**associated constant** on the enum's `impl` block:

```rust
impl EMsg {
    /// Alias of [`EMsg::BaseGeneral`].
    pub const GenericReply: EMsg = EMsg::BaseGeneral;
}
```

The counts line up exactly with the C# original:

| | count |
|---|---|
| enums generated | **91** |
| real Rust variants | **3054** |
| alias `const`s | **39** |
| C# enum members | **3093** (3054 + 39) |

`crates/steamkit-tools/verify_steam_language.py` asserts this equivalence mechanically: it parses the
`(name, value)` pairs out of both the C# `SteamLanguage.cs` and the generated Rust file, resolves the
Rust aliases back to the value of the variant they point at, and then compares the two member sets
member by member, plus the underlying type and the `flags` flag of every enum.

Two consequences to keep in mind:

* `EMsg::from_value(n)` returns the **first declared** variant for a duplicated value — which is
  exactly what a C# `(EMsg)n` cast yields. The alias is reachable only as `EMsg::GenericReply`.
* `ALL` contains **every** member including the aliases, so `ALL.len()` is 3093-ish per enum, not the
  variant count.

#### `crates/steamkit-tools/src/lib.rs` — 58 lines
**C# counterpart:** `Resources/SteamLanguageParser/Program.cs` (the call structure), plus the module layout of the three other C# tool projects.
**Purpose:** The crate root. It declares the four tool families as modules (`analyzer`, `error`, `generator`, `nethook`, `proto_dumper`, `proto_error`, `proto_gen`, `tokenizer`) and offers two small conveniences: `parse_file`, which reads a `.steamd` file, tokenizes it and analyses it in one call, and the two default-path helpers.
**How it works:** `parse_file(entry, language_dir)` reads the entry file to a `String`, calls `tokenize(&text, &entry.display().to_string())` and then `analyze(&tokens, language_dir)`; `language_dir` is the directory against which `#import` paths are resolved. `default_language_dir()` returns `CARGO_MANIFEST_DIR/lang`, i.e. the seven Valve definition files that ship inside the crate. `default_output_file()` returns `CARGO_MANIFEST_DIR/../steamkit/src/base/generated/steam_language.rs`. The module doc-comment carries the C#→Rust mapping table (see the crate overview) and the crate re-exports `analyze`, `ClassNode`, `EnumNode`, `Member`, `Node`, `Root`, `TypeRef`, `LangError`, `LangResult`, `Generator`, `ProtoError`, `ProtoResult`, `tokenize`, `Token`, `TokenKind`.
**Notes and gotchas:** The `.steamd` half of the crate has **no third-party dependencies at all** — `std` only. The `prost`/`prost-build` dependencies exist solely for the protobuf tooling, and `steamkit`/`steamkit-protos` solely for the NetHook analyzer. `default_output_file()` resolves through `..`, so it only works from inside the workspace layout; a standalone checkout of the crate would need `--out`.

#### `crates/steamkit-tools/src/tokenizer.rs` — 292 lines
**C# counterpart:** `Resources/SteamLanguageParser/Parser/LanguageParser.cs`
**Purpose:** Hand-written lexer for the SteamLanguage definition language. It is a faithful port of the C# implementation, which used a single regular expression with named capture groups.
**How it works:** `tokenize(text, file_name) -> LangResult<Vec<Token>>` walks the input as a `Vec<char>` and, at every position, tries the alternatives in the *same order* as the C# regex:

```
(?<whitespace>\s+)
|(?<terminator>[;])
|["](?<string>.+?)["]
|\/\/(?<comment>.*)$
|(?<identifier>-?[a-zA-Z_0-9][a-zA-Z0-9_:.]*)
|[#](?<preprocess>[a-zA-Z]*)
|(?<operator>[{}<>\]=|])
|(?<invalid>[^\s]+)
```

The token kinds are `TokenKind::{Terminator, Str, Identifier, Preprocess, Operator, Invalid}`; each `Token` carries `kind`, `value` (without the delimiters for strings and directives), `file`, `line` and `column` (1-based). `scan_string` implements the lazy `.+?` semantics (it stops at the first closing quote, refuses to span a line break, and requires at least one character — an unterminated literal therefore falls through to the `invalid` group, as in C#). `scan_identifier` allows one optional leading `-` and then `[a-zA-Z_0-9]` followed by `[a-zA-Z0-9_:.]*`. The `#name` directive only captures ASCII letters. `advance()` keeps the line/column counters in sync. `error_at(token, message)` is a crate-internal helper that builds a positional `LangError::Parse`.
**Notes and gotchas:** The C# regex assigns `whitespace` group #1 and the C# loop only enqueues groups with an index **greater than one**, so whitespace never becomes a token — the port reproduces that by simply not emitting a token for whitespace. The `operator` group is `{ } < > ] = |`; note that `[` is *not* an operator and therefore lands in the `invalid` group. `char::is_whitespace` is the Unicode `White_Space` property, a superset of .NET's `\s` for every character that occurs in Valve's files. Comments are skipped entirely (the C# code explicitly `continue`s on the `comment` group).

#### `crates/steamkit-tools/src/analyzer.rs` — 543 lines
**C# counterpart:** `Resources/SteamLanguageParser/Parser/TokenAnalyzer.cs` and `Resources/SteamLanguageParser/Parser/SymbolLocator.cs`
**Purpose:** Recursive-descent parser that turns a token stream into the node tree the generator consumes, plus the symbol resolution the C# tool did with `SymbolLocator`.
**How it works:** `analyze(tokens, language_dir) -> LangResult<Root>` drives a `Parser` cursor over the tokens. A `Preprocess` token whose value is `import` is followed by a string literal naming the imported file; `import_file()` resolves it against `language_dir` (falling back to the path as given), reads it, **tokenizes and analyses it recursively**, and then splices the imported root's children into the current root. `Identifier` tokens whose value is `class` or `enum` are parsed by `parse_class` / `parse_enum`. `parse_enum` accepts an optional `<TYPE>` underlying type and an optional `flags` keyword. `parse_class` accepts an optional `<Enum::Member>` binding (resolved through `resolve_full_ident`), an `expects Parent` clause (parsed and then **discarded**, exactly as in the C# where the `SymbolLocator` call for the parent is commented out) and an optional `removed` marker. `parse_inner_scope` parses the members: an optional `<opt>` marshal option, then up to two identifiers, then an optional `=` value list (terms split on `|`), then the mandatory `;`, then optional `obsolete` / `removed` markers with an optional quoted reason. `resolve_weak_ident` / `resolve_full_ident` / `identifier_head` are the ports of `SymbolLocator.LookupSymbol`: an identifier that contains `::` is split into class and member, the class must exist in the root, and the member must exist inside it, otherwise a positional parse error is raised. The node model is `Root { children: Vec<Node> }` with `Node::{Enum(EnumNode), Class(ClassNode)}`; `EnumNode` holds `name`, `flags`, `underlying`, `members`; `ClassNode` holds `name`, `removed`, `ident`, `members`; `Member` holds `name`, `type_ref`, `flags`, `flags_opt`, `values`, `obsolete`, `removed`.
**Notes and gotchas:** `removed` members are dropped by the generator, and they are also **excluded from the C# auto-increment** — the generator continues the sequence from the last *emitted* member, not from the last declared one. `obsolete` members are kept. The C# `SymbolLocator` used a regex `[a-zA-Z0-9_:]*` for the identifier head; `identifier_head` reproduces exactly that character class, which means a trailing `:` in `Class::Member::` style input is swallowed. `type_index(root)` / `NodeKind` are a small convenience API that is not used by the generator itself.

#### `crates/steamkit-tools/src/generator.rs` — 1060 lines
**C# counterpart:** `Resources/SteamLanguageParser/Generator/CSharpGen.cs` and `Resources/SteamLanguageParser/CodeGenerator.cs`
**Purpose:** The Rust code emitter. It reproduces the C# generator's *output semantics* — not its syntax — and adds the Rust-specific machinery the C# did not need (associated alias constants, `TryFrom`/`From` conversions, `ALL` lists, flag helpers).
**How it works:** `Generator::new(root, source_label)` holds the merged `Root` and the label used in the `Source:` doc comments. `generate()` splits the root into enums and classes, builds a **global literal table** (`literals`: member name → value, for every member of every enum that has exactly one literal value), resolves every enum and every class constant, then emits the header, the enums and the structs.

*Value resolution.* `resolve_enum` walks the members in declaration order. A member without an explicit value continues from the previous member (`previous + 1`, starting at 0). A member with a value goes through `eval_expression`, which computes the bitwise-or of its terms; a term is looked up first as an integer literal, then in the *current enum's* table (so `Foo = A | B` works for previously declared members of the same enum), then in the global literal table (so a member of another enum can be referenced by name). The resolved value is range-checked against the underlying type and an out-of-range member is a hard `LangError::Unsupported`. Each resolved member records `primary: bool` — `true` only for the first member that carries that value. That flag is the whole basis of the alias mechanism.

*Emission.* `emit_enum` writes the doc comment, `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]`, `#[repr(<underlying>)]` and the variants (skipping non-primary members; a variant with an `obsolete` reason gets `#[deprecated]` or `#[deprecated = "..."]`). `emit_enum_impl` then writes, inside `impl <Enum> { ... }`: one `pub const <Alias>: <Enum> = <Enum>::<Primary>;` per non-primary member, the `ALL: &'static [Self]` slice (which contains **every** member, aliases included), a `pub const fn value(self) -> <underlying>`, and — for `flags` enums — `bits()`, `is_empty()`, `contains()` and `union()`/`intersect()`. `emit_enum_conversions` writes `TryFrom<<underlying>>`, an additional `TryFrom<u32>` when every value is non-negative and fits in `u32` (this is the wire representation of a message id) and `From<Enum> for <underlying>`.

*Structs.* `resolve_all_consts` resolves the `const` members of the classes in two passes: first the ones with a literal value, then the ones that refer to another constant by its `Class::CONST` name. `emit_structs` opens a `pub mod internal { ... }` block (the C# put these types in the `SteamKit2.Internal` namespace; the Rust port keeps them in a nested module so the enums stay at the root of `steam_language`) and emits one `pub struct` per class. `rust_field_type` maps the marshal flags: `proto` → `Vec<u8>`, `boolmarshal` → `bool`, `steamidmarshal`/`gameidmarshal` → `u64`, and a numeric `flags_opt` (`byte<20>`) → `Vec<<base>>`. Each struct also gets an `impl Default` whose values mirror the C# constructor (zero for a plain field, `false` for a bool, an empty `Vec`, the variant with the value zero — falling back to the first declared variant — for an enum, and the special values `ulong.MaxValue` → `u64::MAX` etc. for the `MaxValue`/`MinValue` spellings). `emit_error_type` writes the shared `UnknownEnumValue` error struct and its `From<UnknownEnumValue> for crate::error::SteamError` conversion.

*Names.* `rust_variant_name` keeps the Valve spelling verbatim and only escapes Rust keywords with `r#`. `rust_field_name` converts to `snake_case` via `to_snake_case` and escapes keywords the same way. `to_screaming_snake_case` is used for the associated constants of the structs.
**Notes and gotchas:** The generated file is `crates/steamkit/src/base/generated/steam_language.rs` (18 877 lines, 715 KB). Because the generated struct names come from Valve (`MsgClientJustStrings`, `ExtendedClientMsgHdr`, …) they are not `snake_case`, hence the file-level `#![allow(non_camel_case_types)]`. The C# generator emitted two files (`SteamLanguage.cs` and `SteamLanguageInternal.cs`); the Rust generator emits **one** file with the structs inside `pub mod internal`. `#[derive(PartialOrd, Ord)]` is emitted for every enum, which is safe only because every enum has a single underlying primitive type. The `ALL` slice and the alias constants mean `EMsg::ALL` is not the same thing as the set of variants — code that iterates `ALL` will see duplicates.

#### `crates/steamkit-tools/src/error.rs` — 84 lines
**C# counterpart:** none — the C# tools threw plain `System.Exception` with a formatted message; the port needed a typed error to travel through `Result`.
**Purpose:** The error type of the SteamLanguage (`.steamd`) toolchain.
**How it works:** `LangError` is a three-variant enum: `Io { path, source }` (a definition file could not be read), `Parse { file, line, column, message }` (the token stream did not match the grammar) and `Unsupported { file, message }` (a construct the port does not implement — e.g. a value that does not fit into the underlying type, an unknown underlying type, or an unresolvable value term). `LangError::parse(file, line, column, message)` and `LangError::unsupported(file, message)` are the constructors. `Display` renders the parse error as `file:line,column: message`, matching the format of the C# exception message (`Expecting operator '>' but got 'x' at <file> <line>,<col>-<line>,<col>`). `source()` exposes the underlying `std::io::Error`.
**Notes and gotchas:** The `Unsupported` variant is deliberately distinct from `Parse`: an out-of-range enum value is a *generator* limitation, not a syntax error, and the message names the enum, the member and the target type.

#### `crates/steamkit-tools/src/main.rs` — 165 lines
**C# counterpart:** `Resources/SteamLanguageParser/Program.cs`
**Purpose:** The `steamkit-lang-gen` binary — the command-line front end of the `.steamd` generator.
**How it works:** `parse_args` accepts `--lang-dir <dir>`, `--entry <file>`, `--out <file>` and `--stdout`, in both the `--flag value` and `--flag=value` spellings; `--help`/`-h` prints the usage text and exits successfully. The defaults are: language dir = `default_language_dir()` (the crate's `lang/`), entry = `steammsg.steamd`, output = `default_output_file()`. `--stdout` clears the output path so the generated code goes to standard output. `run` resolves the entry against the language dir, checks that it exists, calls `parse_file`, builds a `Generator` with the source label `lang/<entry>` and calls `generate()`. When writing to a file it creates the parent directory first, writes the code and prints `Wrote <path> (<n> enums, <m> lines)`, where `<n>` is counted by scanning the generated text for lines that start with `pub enum `.
**Notes and gotchas:** The C# `Program.cs` took the *SteamRE project root* as its single argument and hard-coded `Resources/SteamLanguage/steammsg.steamd` plus the output paths; the Rust binary takes the language directory and the output file explicitly, which is what makes it usable from a workspace where the paths differ. The C# also emitted two files (enums and internal structs); the Rust binary emits one.

---

#### 06.3 `steamkit-proto-gen` — regenerating the `prost` code

#### The protobuf to Rust pipeline

```
vendor/Protobufs/<dir>/*.proto            Valve's upstream protos (read only)
   │  tools/stage_protos.py
   ▼
crates/steamkit-protos/proto/<dir>/*.proto patched, vendored copies
crates/steamkit-protos/proto/manifest.txt  "<dir> <file.proto> ..." per line
   │  crates/steamkit-protos/build.rs   (protoc → descriptor set → prost → OUT_DIR)
   ▼
crates/steamkit-protos/src/lib.rs         include!(OUT_DIR/<set>/<set>.rs)
```

`tools/stage_protos.py` is the staging step, and it exists because the C# build's protoc invocation
(`Resources/ProtobufGen/generate-all.ps1`) differs from a naive one in three ways:

1. **Valve's non-standard file option.** `option additional_includes = "...";` is a Valve-internal
   *file* option that is not part of standard protobuf. `protoc` rejects it with *"Some options could
   not be correctly parsed"*. The script strips the whole line with
   `ADDITIONAL_INCLUDES_RE = ^\s*option\s+additional_includes\s*=.*;\s*$`. It is a file-level
   directive about where Valve's build finds extra headers; it has **no effect on the wire format**.
2. **Two modern field options.** `protoc` 36.2 also rejects the field options `boxed_type` and
   `synthetic_default`. `stage_protos.py` removes them from every `[...]` option list
   (`FIELD_OPTIONS_WITHOUT_WIRE_EFFECT`). Both are code-generation hints — one asks for a boxed
   representation in the target language, the other for a synthetic default value — and **neither
   changes the encoding or the decoding of the message**, so dropping them cannot change the wire
   format.
3. **Include roots.** Valve's protos import each other with bare file names, so the protoc include
   root must be the per-directory `ProtoDir` itself, not the `Protobufs` root. The script also
   deliberately does **not** stage Valve's vendored `google/protobuf/*.proto` copies: they are an
   ancient protobuf-3.x fork, and `protoc` 36 refuses to parse a `descriptor.proto` that differs from
   its own compiled-in one. The build passes `protoc`'s own include directory instead.

The script copies the CSV-listed protos of `Resources/ProtobufGen/protos.csv` plus their transitive
import closure, writes `manifest.txt`, copies the one-off `gc.proto` from `Resources/ProtobufGen`, and
finally validates every staged directory by running `protoc` on it.

`crates/steamkit-protos/build.rs` then reads `manifest.txt` and, for every set, runs
`protoc -I<protoc include> -I<set dir> --descriptor_set_out=<set>.desc --include_imports <files>`,
decodes the descriptor set with `prost_types::FileDescriptorSet` and calls
`prost_build::Config::compile_fds` with `out_dir = OUT_DIR/<set>` and
`default_package_filename(<set>)`. The generated code lands in `OUT_DIR` and is pulled into
`crates/steamkit-protos/src/lib.rs` with `include!`.

`steamkit-proto-gen` is the **standalone** equivalent of that build-script step: it compiles a set of
`.proto` files with the same vendored `protoc` and generates the same `prost` code, but writes it to a
file (or stdout) instead of `OUT_DIR`. It is the tool you use when you want to inspect or diff the
generated code, or when you want to generate for a directory that is not part of the build.

#### `crates/steamkit-tools/src/proto_gen/mod.rs` — 7 lines
**C# counterpart:** `Resources/ProtobufGen/Program.cs` and `Resources/ProtobufGen/ProtobufGen/ProtobufGen.csproj`
**Purpose:** Module root of the protobuf generator; it declares `code_generator` and `proto_set` and re-exports their public API.
**How it works:** `pub mod code_generator; pub mod proto_set;` plus `pub use code_generator::{RustCodeGenerator, SteamServiceGenerator};` and `pub use proto_set::{proto_files_in, protoc_include_path, protoc_path, ProtoSet};`. The module doc-comment maps the C# files to the Rust ones (`Program.cs` → the binary, `SteamKitCSharpCodeGenerator.cs` → `code_generator`, `ProtoBuf.Reflection.FileDescriptorSet` → `proto_set`).
**Notes and gotchas:** The C# generator project was a *library* (`ProtobufGen.dll`) that was consumed by the SteamKit build; the Rust one is a plain module inside the tools crate plus a thin binary.

#### `crates/steamkit-tools/src/proto_gen/proto_set.rs` — 219 lines
**C# counterpart:** `Resources/ProtobufGen/ProtobufGen/Program.cs` (the `FileDescriptorSet` handling) — the Rust counterpart of protobuf-net's own `.proto` parser.
**Purpose:** Compiles `.proto` files into a protobuf descriptor set and hands the individual file descriptors back.
**How it works:** `protoc_path()` and `protoc_include_path()` return the paths of the **vendored** `protoc` binary and its include directory via `protoc_bin_vendored`, so the tools work on a machine with no protobuf compiler installed. `proto_files_in(directory)` lists the `*.proto` files of a directory (case-insensitive extension match) and sorts them by name. `ProtoSet::compile(protos, includes)` builds a `protoc` command line — `-I<vendored include>`, then one `-I` per extra include, then one `-I` per parent directory of the compiled files, then `--descriptor_set_out=<tmp>/descriptor.bin --include_imports` and the files themselves — runs it, and on failure reports `ProtoError::Compile` with `protoc`'s stderr. The temporary directory is `std::env::temp_dir()/steamkit-proto-gen-<pid>-<counter>` and is removed afterwards (the counter is a `static AtomicU32` so concurrent invocations in one process do not collide). `split_descriptor_set` does **not** decode the set with `prost`; it walks the wire format with `proto_dumper::wire::parse_message`, keeps the field-1 (`repeated FileDescriptorProto file`) payloads byte for byte and hands each to `RawDescriptor::from_bytes`. That preserves the fields `prost` does not know about — which is exactly where protobuf options declared as extensions live.
**How it works (continued):** `ProtoSet::file(name)` / `file_by_path(path)` look a descriptor up by its virtual name (`steammessages_base.proto`) or by the file-name part of a path. `ProtoSet::closure(root)` returns a file plus everything it imports, ordered so that a file always comes *after* the files it imports; `collect` walks the dependency graph depth-first with a `visited` list.
**Notes and gotchas:** The C# tool shipped its **own** `.proto` parser; the Rust port deliberately drives `protoc` instead, so that the descriptors the tool inspects are the same bytes the generated `prost` types are built from. `file_by_path` compares only the file-name component, so it matches on the virtual name rather than on the on-disk path.

#### `crates/steamkit-tools/src/proto_gen/code_generator.rs` — 244 lines
**C# counterpart:** `Resources/ProtobufGen/ProtobufGen/SteamKitCSharpCodeGenerator.cs`
**Purpose:** Turns a compiled descriptor set into Rust source.
**How it works:** `RustCodeGenerator::new(namespace)` holds the optional namespace. `generate(files)` validates the namespace (`validate_namespace`: it must be non-empty and every dot-separated part must be a valid identifier), maps each `RawDescriptor` to a `prost_build::Module` derived from its protobuf package, installs a custom service generator (`config.service_generator(Box::new(SteamServiceGenerator::default()))`) and calls `config.generate(requests)`. `compose(modules, namespace)` then assembles the per-module code into a single file: the modules are sorted by their `Module` path, a stack of open `pub mod` blocks is maintained, and the result is wrapped in the module path that the namespace maps to (`namespace_to_module_path` → `to_snake` per part). The file starts with the marker `// This file is @generated by steamkit-proto-gen.`.

`SteamServiceGenerator` implements `prost_build::ServiceGenerator`. For every Steam unified-messaging service it writes a **trait** rather than a class: `pub trait <ServiceName>` with a `const SERVICE_NAME: &'static str` (the wire name), and per RPC a `const <METHOD>_NAME: &'static str = "<Service>.<Method>#1";` plus a `fn handle_<method>(&mut self, message: &<Payload>);`. A method whose output type is `NoResponse` is treated as a **notification** (its payload is the input type); every other method answers with the response type. `is_notification` compares the output type against `NoResponse` after stripping the leading dot.
**Notes and gotchas:** The C# generator emitted a `UnifiedService` subclass per service with a
`HandleResponseMsg`/`HandleNotificationMsg` dispatcher; the Rust generator emits the same information
as a trait instead, because the port's Steam client dispatches through the trait. The C# generator
**did not** emit protobuf extensions at all, and neither does `prost_build` here. The `--namespace`
argument of the C# tool became the C# namespace of the generated classes; in Rust it becomes the
module path the types live in (and, for files that declare no package, it is what determines the
package of the file).

#### `crates/steamkit-tools/src/bin/steamkit-proto-gen.rs` — 225 lines
**C# counterpart:** `Resources/ProtobufGen/ProtobufGen/Program.cs`
**Purpose:** The `steamkit-proto-gen` command-line front end.
**How it works:** Options: `--namespace <ns>`, `--proto <file>` **or** `--proto-dir <dir>` (mutually exclusive, one of them required), `--output <file>` (default: stdout), `--include <dir>` (repeatable) and `--stdout`. `run` resolves the input: for `--proto` the file's own directory becomes an include root; for `--proto-dir` every `*.proto` of the directory is compiled together with the directory as the include root. The extra `--include` directories are appended. `ProtoSet::compile` then runs `protoc`. Which files are handed to the generator depends on the mode: with `--proto` only that file **plus its transitive import closure** (`set.closure(name)`, ordered dependencies-first) is generated; with `--proto-dir` every compiled file is generated. The result is written to `--output` (creating the parent directory first) or printed to stdout.
**Notes and gotchas:** With `--proto`, the closure is what makes a single-file invocation usable: the generated Rust code needs the types of the imported files, so `prost` must see them. With `--proto-dir`, everything in the directory is emitted, which is the mode that mirrors the per-directory sets of `manifest.txt`.

#### `crates/steamkit-tools/src/proto_error.rs` — 94 lines
**C# counterpart:** none — the C# tools reported failures as console output and non-zero exit codes.
**Purpose:** The shared error type of the protobuf tooling (`steamkit-proto-gen` and `steamkit-proto-dump`).
**How it works:** `ProtoError` has six variants: `Io { path, source }`, `Protoc { message }` (the vendored `protoc` could not be located or run), `Compile { file, message }` (a `.proto` did not compile), `Decode { message }` (a descriptor could not be decoded), `NotFound { path }` and `InvalidArgument { message }`. The constructors `io`, `not_found` and `invalid_argument` are provided. `Display` renders `protoc failed: …` for `Protoc`, `unable to decode descriptor: …` for `Decode`, and the raw path for `NotFound`.
**Notes and gotchas:** `ProtoError` is deliberately separate from `LangError` and `NetHookError`: the three tool families fail in different ways and share no code.

---

#### 06.4 `steamkit-proto-dump`

`steamkit-proto-dump` reverses the direction of `steamkit-proto-gen`: it takes protobuf **descriptors**
and writes them back out as `.proto` **source**. Two sources of descriptors are supported:

* a **binary** (a Steam client executable), whose embedded `FileDescriptorProto` messages are found
  by scanning for the length-prefixed file name that every descriptor starts with;
* a **directory of `.proto` files** (`--protos <dir>`), which are compiled with `protoc` and then
  dumped again. This mode is a **Rust addition** — the C# `ProtobufDumper` could only read the
  definitions out of a binary, so a round trip (`dump → stage → compile → dump`) was impossible
  without the original binary. It is the mode to use to normalise or to inspect the staged protos.

#### `crates/steamkit-tools/src/proto_dumper/mod.rs` — 19 lines
**C# counterpart:** the file layout of `Resources/ProtobufDumper/ProtobufDumper/`
**Purpose:** Module root of the protobuf dumper.
**How it works:** Declares `executable_scanner`, `protobuf_collector`, `protobuf_dumper`, `raw_options`, `util` and `wire`, and re-exports `scan_file`, `scan_path`, `CandidateResult`, `ProtobufCollector`, `ProtobufDumper`, `RawDescriptor`, `RawFileDescriptor`, `RawMessage`, `to_literal`, `to_literal_bytes`, `to_literal_double`, `to_literal_float` and `OptionList`. The doc-comment carries the C#→Rust map, including the note that `ProtobufDumper/Descriptor.cs` is replaced by `prost_types` plus the `raw_options` module.
**Notes and gotchas:** `prost_types` is the Rust counterpart of the hand-written `Descriptor.cs`: both are the generated code of `google/protobuf/descriptor.proto`.

#### `crates/steamkit-tools/src/proto_dumper/wire.rs` — 244 lines
**C# counterpart:** none — the C# tool used protobuf-net, which keeps unknown fields natively; the Rust port has to read the wire format itself.
**Purpose:** A minimal protobuf wire-format reader.
**How it works:** `WireValue` is a four-variant enum (`Varint(u64)`, `Fixed32(u32)`, `Fixed64(u64)`, `Bytes(Vec<u8>)`) with `as_varint`/`as_fixed32`/`as_fixed64`/`as_bytes` accessors; `WireField` pairs a field number with a value. `parse_message(data)` walks the message: a key varint gives `number = key >> 3` and `wire_type = key & 7`; field number 0 is rejected; wire types 0/1/2/5 are supported and 3/4 (groups) and 6/7 are rejected with `ProtoError::Decode`. `read_varint` rejects a varint longer than 64 bits and a truncated one. `consume_one_message(data)` returns the length of the **first** protobuf message in the buffer: it walks the fields and stops at the second field with number 1 (the `name` field of a `FileDescriptorProto`), which is how the boundary between two concatenated descriptors is found.
**Notes and gotchas:** This module exists for exactly one reason: `prost` **drops the fields it does not
know** when it decodes a message, and the unknown fields of a descriptor are exactly where the
protobuf options that are declared as *extensions* live — the C# dumper, using protobuf-net, could
still print them. Reading the wire format directly is the only way to keep them.

#### `crates/steamkit-tools/src/proto_dumper/raw_options.rs` — 441 lines
**C# counterpart:** `Resources/ProtobufDumper/ProtobufDumper/Descriptor.cs` (the part that protobuf-net kept the unknown fields of).
**Purpose:** A raw, field-number-addressable view of a protobuf descriptor.
**How it works:** `RawDescriptor` pairs a decoded `FileDescriptorProto` (`prost_types`) with the **bytes it was decoded from**. `RawMessage` is the base view: `parse(data)` turns a byte slice into `Vec<WireField>` (a parse failure yields an empty message rather than an error), and offers `get(number)`, `values(number)`, `bytes(number)`, `message(number)` and `messages(number)`. On top of it sit thin wrappers with the field numbers of the descriptor schema spelled out as constants: `file_fields` (`MESSAGE_TYPE = 4`, `ENUM_TYPE = 5`, `SERVICE = 6`, `EXTENSION = 7`, `OPTIONS = 8`), `message_fields` (`FIELD = 2`, `NESTED_TYPE = 3`, `ENUM_TYPE = 4`, `EXTENSION_RANGE = 5`, `EXTENSION = 6`, `OPTIONS = 7`, `ONEOF_DECL = 8`), `field_fields` (`OPTIONS = 8`), `enum_fields` (`VALUE = 2`, `OPTIONS = 3`), `enum_value_fields` (`OPTIONS = 3`), `service_fields` (`METHOD = 2`, `OPTIONS = 3`), `method_fields` (`OPTIONS = 4`), `oneof_fields` (`OPTIONS = 2`) and `extension_range_fields` (`OPTIONS = 3`). The wrappers are `RawFileDescriptor`, `RawMessageDescriptor`, `RawFieldDescriptor`, `RawEnumDescriptor`, `RawEnumValueDescriptor`, `RawServiceDescriptor`, `RawMethodDescriptor`, `RawOneofDescriptor` and `RawExtensionRange`, each exposing only the accessors its owner needs (`options()`, `fields()`, `nested_types()`, `enums()`, `extensions()`, `extension_ranges()`, `oneofs()`, `values()`, `methods()`).
**Notes and gotchas:** Every wrapper holds only the `RawMessage`; the typed `prost_types` descriptor is
kept alongside it in `RawDescriptor`. The dumper reads the **typed** descriptor for structure and the
**raw** one for options, so the two must be kept in sync — they are, because both come from the same
bytes.

#### `crates/steamkit-tools/src/proto_dumper/executable_scanner.rs` — 120 lines
**C# counterpart:** `Resources/ProtobufDumper/ProtobufDumper/ExecutableScanner.cs`
**Purpose:** Finds the protobuf descriptors that are embedded in a Steam binary.
**How it works:** Every embedded `FileDescriptorProto` starts with field 1 (`name`), so the scanner searches for the byte `0x0A` (the length-delimited marker of field 1) and then reads the varint length that follows — one byte, or two if the continuation bit is set. If the length does not fit in the buffer the candidate is skipped. The length-prefixed bytes are decoded as ASCII (`Encoding.ASCII` semantics: every byte above `0x7F` becomes `?`) and the name must end in `.proto` (case-insensitive) and match `^[a-zA-Z_0-9\\/.-]+\.proto$` (`is_valid_proto_name`); a name that ends in `.proto` but fails the pattern is reported as "Skipping potentially valid '<name>'". The callback `process_candidate(name, bytes)` receives the descriptor name and the bytes **from the marker onwards**, and returns the number of bytes the descriptor occupies (or `None` to keep scanning).
**Notes and gotchas:** The scanner hands the candidate the bytes starting at the `0x0A` marker — i.e. the
full serialized `FileDescriptorProto`, which is exactly what `RawDescriptor::from_bytes` expects. The
two-byte length form is handled, but a third continuation byte is treated as "not a candidate" and the
scan advances by one byte.

#### `crates/steamkit-tools/src/proto_dumper/protobuf_collector.rs` — 65 lines
**C# counterpart:** `Resources/ProtobufDumper/ProtobufDumper/ProtobufCollector.cs`
**Purpose:** Collects the descriptors that the scanner found.
**How it works:** `ProtobufCollector` holds a `Vec<RawDescriptor>`. `try_parse_candidate(data)` calls `wire::consume_one_message(data)` to find the length of the descriptor; a length of 0 is an immediate `CandidateResult::Invalid` ("No data was consumed"). Otherwise the slice is handed to `RawDescriptor::from_bytes`; success pushes the descriptor and returns `CandidateResult::Ok` plus the consumed byte count, failure returns `CandidateResult::Invalid` with the error message. `decode_descriptor(data)` is a convenience wrapper around `FileDescriptorProto::decode`.
**Notes and gotchas:** The candidate bytes are **not** trimmed to the message length before the parse —
`RawDescriptor::from_bytes` decodes the whole slice, and protobuf's own decoder stops at the end of
the message, so the extra bytes are harmless.

#### `crates/steamkit-tools/src/proto_dumper/protobuf_dumper.rs` — 1918 lines
**C# counterpart:** `Resources/ProtobufDumper/ProtobufDumper/ProtobufDumper.cs`
**Purpose:** Turns the collected descriptors back into `.proto` source.
**How it works:** The dumper keeps four indexes: `protobuf_map` (a `ProtoNode` per file: name, descriptor, dependencies, public dependencies, the types it uses, and whether it was found), `protobuf_map_index` (name → index), `missing_files` and `protobuf_type_map` / `protobuf_type_map_index` (a `ProtoTypeNode` per fully qualified type, with the `TypeSource` that declared it: `Extension`, `Enum`, `Message` or `Service`).

*`analyze()`* walks every descriptor, records the file and all the types it declares or refers to (including nested types, enum values, extensions, extendees, service methods and their input/output types), then calls `collect_dependencies()` — which skips dependencies whose name starts with `google` and records every other unknown dependency as *missing* — and finally checks every file: a file whose dependencies or types cannot be resolved makes `analyze()` return `false` after printing `Not all dependencies were found for <file>` / `Dependency not found: <file>` or `Not all types were resolved for <file>` / `Type not found: <type>`. `add_public_dependencies` then computes, per file, the set of files whose types it may refer to (following `public_dependency` transitively).

*`dump_files(callback)`* renders every descriptor and hands the file name and the text to the callback.

*`dump_file_descriptor`* writes, in order: the `syntax`, the `import` statements (with the `public` / `weak` modifiers), the `package`, the file options, the top-level extensions, the enums, the messages and the services. The imports that the **options** need are collected in `option_imports` and inserted into the import block afterwards (this is how `option (foo) = …` referring to an extension of another file produces the required `import "…";`).

*Options.* `dump_file_options`, `dump_field_options`, `dump_message_options`, `dump_enum_options`, `dump_enum_value_options`, `dump_service_options`, `dump_method_options`, `dump_oneof_options` and `dump_extension_range_options` all funnel into `dump_options_matching`, which does two things: it walks the fields of the *options message* of the descriptor schema (`google.protobuf.FileOptions` etc.) and prints the ones the schema declares but that the standard dump did not already print (`standard_option_fields(type_name)` lists the field numbers that the schema knows), and it collects the **extensions** whose `extendee` is the options message and that live in one of the file's public dependencies. `dump_options_field_recursive` descends into a message-typed option and prints its sub-fields as `key.sub_field = value`. `get_option_values` formats the value of one option field according to its protobuf type — `Int32`/`Sint32`/`Sfixed32`/`Uint32`/`Fixed32`/`Int64`/`Sint64`/`Sfixed64`/`Uint64`/`Fixed64` as numbers (with zig-zag decoding for the `sint` variants), `Bool` as `true`/`false`, `Float`/`Double` through the literal helpers, `String`/`Bytes` as quoted literals, `Enum` as the **name** of the enum value (falling back to the number when the value is unknown) and `Message` as an aggregate value. A non-repeated option that occurs more than once keeps only the last occurrence — that is the protobuf merge semantics.

*Structure.* `dump_descriptor` writes a message with its options, its nested extensions, its nested messages (skipping the synthetic `map_entry` ones), its nested enums, its fields (with `repeated`/`optional`/`required`, the resolved type name, the default value and the field options) and its reserved ranges. `dump_enum_descriptor` writes the enum values and the reserved ranges. `dump_service` writes the methods with their input/output types and options.
**Notes and gotchas:** `standard_option_fields` exists because the C# implementation read the options that
the descriptor schema knows from the **typed** properties of the options message and only the ones
the schema of the binary does not know from the raw message; the Rust port does the same split. The
`map_entry` messages are skipped because `protoc` synthesises them and Valve's files do not contain
them. The output is intended to be re-compilable: the dumper re-adds the imports that the options
require.

#### `crates/steamkit-tools/src/proto_dumper/util.rs` — 119 lines
**C# counterpart:** `Resources/ProtobufDumper/ProtobufDumper/Util.cs`
**Purpose:** Literal formatting helpers.
**How it works:** `to_literal(input)` formats a protobuf string literal: the short escapes `\a \b \f \n \r \t \v`, a backslash escape for `\` and `"`, and a **three-digit octal escape** for everything below `0x20` and for `0x7F`. `to_literal_bytes(input)` formats a bytes literal with the same rules, but escapes only `\` and `"` and octal-escapes every byte below `0x20` or at/above `0x7F`. `to_literal_double` / `to_literal_float` render `inf`, `-inf` and `nan` for the non-finite values. `OptionList` is an ordered list of `(key, value)` pairs with `add`, `extend`, `len`, `is_empty` and `iter`.
**Notes and gotchas:** The escaping rules are byte-for-byte the C# ones, so a dumped `.proto` file
round-trips through `protoc` byte-identically to the original in the common case.

#### `crates/steamkit-tools/src/bin/steamkit-proto-dump.rs` — 374 lines
**C# counterpart:** `Resources/ProtobufDumper/ProtobufDumper/Program.cs`
**Purpose:** The `steamkit-proto-dump` command-line front end.
**How it works:** The argument parsing is positional, like the C# one: `steamkit-proto-dump [-v] [-dump] [--stdout] <target>... [<output-dir>]`. The **last** unnamed argument is the output directory, the others are the binaries to scan; a target that does not exist prints `Could not find file <x>`, an output directory that exists as a file prints `Output directory path is not valid <x>`. `-v` prints `ProtobufDumper version: <crate version>`; `-dump` additionally writes the raw bytes of every candidate to `<out>/<name>.dump`. The default output directory is the file stem of the first target.

`--protos <dir>` switches to the directory mode: every `.proto` of the directory is compiled with `ProtoSet::compile` (the directory is the include root) and the resulting descriptors are handed to the same `ProtobufDumper`. The output directory defaults to the stem of the input directory.

Both modes then run `ProtobufDumper::new(descriptors)`, `analyze()` (a failure prints
`Dump failed. Not all dependencies and types were found.` and returns a failure) and `dump_files(callback)`, where the callback writes `<out>/<name>` or, with `--stdout`, prints `// <name>` followed by the text. Progress lines are `Loading binary '<target>'...`, `<name>... OK!` / `<name>... is invalid: <error>`, `  ! Dumping to '<file>'!` and `  ! Outputting proto to '<file>'`. The colours (`green`, `red`) are only emitted when stdout is a terminal (`std::io::IsTerminal`).
**Notes and gotchas:** The C# tool's `--protos` equivalent did not exist; the C# `ProtobufDumper`
could only ever read the definitions out of a binary. The `-dump` flag is what makes the scan
diagnosable: it writes the exact bytes of each candidate, so a descriptor that fails to parse can be
inspected with `protoc --decode_raw`.

---

#### 06.5 `steamkit-nethook-analyzer`

#### The on-disk contract with the NetHook2 DLL

NetHook2 is the Windows library that is injected into the Steam client and writes the traffic it sees
into a directory of capture files. The analyzer is the **consumer** of that directory; the contract is
the *file name*, and it is the only contract.

The NetHook2 logger builds the name with the C++ format string

```
"%03u_%s_%d_%s"      →   {sequence:03}_{direction}_{emsg}_{message name}
```

(`CLogger::GetFileNameBase` in `Resources/NetHook2/NetHook2/logger.cpp`: the sequence number padded to
at least three digits, `"in"` for a packet the client received and `"out"` for one it sent, the
`EMsg` number and the human-readable message name that the sniffer took from the client). The Rust
port of the sniffer (`crates/steamkit-nethook/src/logger.rs`) reproduces it byte for byte with
`format!("{:03}_{}_{}_{}", …)`, and both sides have a unit test that pins the four canonical names
`001_in_2_ProtoBuf`, `001_out_65535_ProtoBuf`, `042_in_0_Null` and `1000_out_1_Name`.

The analyzer does **not** depend on that exact layout. It locates the three meaningful parts with the
equivalent of the C# regular expression

```
(?<num>\d+)_(?<direction>in|out)_(?<emsg>\d+)      (C#: RegexOptions.IgnoreCase)
```

(`NetHookAnalyzer2/NetHookAnalyzer2/NetHookItem.cs`, `[GeneratedRegex(...)]`). The **leftmost match
anywhere in the file name wins**, the direction is matched case-insensitively, and everything after
the message number (the message name and the `.bin` extension) is ignored. So `001_in_2_ProtoBuf.bin`
and `abc12_in_2_Whatever.bin` are both recognised, and a file whose name does not match is simply
"not an item" — the C# tool returns `false` from `LoadFromFile`, the Rust port returns `Ok(None)`.

#### The message reader and the protobuf-header subtlety

A capture file holds one Steam message:

```
[4 bytes] raw EMsg
[header]  depends on the message
[body]
[payload] optional, the bytes that follow the body
```

`read_message` (in `src/nethook/message_reading.rs`) reads the four-byte little-endian message number
and then picks the header:

| Condition | Header | Rust type |
|---|---|---|
| `MsgUtil::is_proto_buf(raw_emsg)` | the protobuf header | `MsgHdrProtoBuf` |
| `EMsg::ChannelEncryptRequest` / `Response` / `Result` | the plain header | `MsgHdr` |
| everything else | the extended client header | `ExtendedClientMsgHdr` |

**The subtlety is where the header starts.** For a protobuf message the header *carries the message
number itself* — `MsgHdrProtoBuf` begins with `protomask EMsg msg`, i.e. the masked 32-bit message
number — so the header reader is started at **offset 0 of the capture**, and the four bytes that were
already read as `raw_emsg` are part of the header. For every other header type the message number is a
*separate* field that precedes the header, so the header reader is started at **offset 4** and the
body starts at `4 + header_reader.position()`.

This is exactly the behaviour of the C# `NetHookItem_MessageReading.ReadFile`, where `PeekUInt` reads
the four bytes and then **seeks back** (`stream.Seek(-sizeof(uint), SeekOrigin.Current)`), so the
header deserialiser always starts at position 0 — which is correct for the protobuf header and would
be wrong for the others, except that the C# deserialisers for `MsgHdr`/`ExtendedClientMsgHdr` expect
to be positioned *after* the message number, so the C# relies on the deserialiser's own framing. The
Rust port makes the two cases explicit instead of relying on that.

A second consequence of the same design: **a protobuf body always consumes the whole remainder of the
capture file**, because a protobuf message of the Steam wire format is not length-prefixed. So
`ReadMessage.payload` is empty for every protobuf message; the payload is only non-empty for the
non-protobuf messages whose header declares a length.

#### `crates/steamkit-tools/src/nethook/mod.rs` — 36 lines
**C# counterpart:** the file layout of `Resources/NetHookAnalyzer2/NetHookAnalyzer2/`
**Purpose:** Module root of the NetHook analyzer.
**How it works:** Declares the fourteen sub-modules and re-exports the public API: `Body`, `ProtobufBody`, `ProtobufField`, `ProtobufValue`, `SerializableBody`, `WireType`, `NetHookDump`, `NetHookError`, `NetHookResult`, `CaptureFileName`, `NetHookItem`, `PacketDirection`, `read_message`, `Header`, `ReadMessage`, `TreeNode`, `default_specializations`, `ExtraObject`, `GameCoordinatorSpecialization`, `Specialization`, `SpecializationContext` and `build_tree`. The doc-comment contains the C#→Rust map for every ported file.
**Notes and gotchas:** The C# tool was a WPF application; `MainForm.cs`, `TreeNodeObjectExplorer.cs`,
`ListViewColumnHider.cs`, `ListViewDoubleBuffered.cs`, `NetHookListViewItemSequentialComparer.cs`,
`StringExtensions.cs`, `TypeExtensions.cs`, `SteamUtils.cs` and the `Properties/` folder have **no**
counterpart — the Rust port is the headless part of the tool only.

#### `crates/steamkit-tools/src/nethook/error.rs` — 81 lines
**C# counterpart:** none — the C# tool caught exceptions at the top of the WPF event handlers and showed them in the tree.
**Purpose:** The error type of the analyzer.
**How it works:** `NetHookError` has four variants: `Io { path, source }` (a capture file could not be read), `Malformed { message }` (the captured data does not follow the Steam wire format), `Protobuf { message }` (a protobuf body could not be decoded) and `Steam { source }` (the `steamkit` library rejected the data). `From<SteamError> for NetHookError` lets the `?` operator cross the crate boundary; `source()` exposes the underlying error for both `Io` and `Steam`.
**Notes and gotchas:** `Malformed` and `Protobuf` are separate so that the tree builder can tell a
framing problem from a decoding problem when it renders
`NetHookError encountered whilst parsing item: …`.

#### `crates/steamkit-tools/src/nethook/item.rs` — 250 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/NetHookItem.cs`
**Purpose:** The capture-file naming contract and the per-packet metadata.
**How it works:** `parse_capture_file_name(name) -> Option<CaptureFileName>` is the hand-written equivalent of the C# regex. It scans for the **leftmost** position at which the whole pattern matches: a run of ASCII digits (the sequence), an `_`, the case-insensitive direction `in`/`out`, an `_` and a run of ASCII digits (the message number). It returns `CaptureFileName { sequence: u32, direction: PacketDirection, emsg: u32 }`; `None` means "not a capture file". `NetHookItem::load_from_file(path)` parses the file name, takes the modification time as the timestamp (falling back to the Unix epoch when the platform cannot report it) and maps the message number to an `EMsg` with `EMsg::from_value(...)`, falling back to `EMsg::Invalid` for an unknown number while keeping the raw number in `raw_emsg`. `emsg_name()` renders the message the way the C# tool does: the `Debug` name of the variant, or the bare number when the Steam language does not know the message. `read_bytes()` reads the capture file; `inner_message_name()` returns the name of the unified-messaging method or the game coordinator message the packet carries.
**Notes and gotchas:** Four unit tests pin the behaviour: the canonical names, the case-insensitive
direction (`001_IN_2_Name.bin`), the leftmost match (`abc12_in_2_Name.bin` → sequence 12) and the
rejection of names that do not match (`not-a-capture.bin`, `001_xx_2_Name.bin`, `001_in_Name.bin`).
`raw_emsg` is kept separately from `emsg` because the file name may carry a message number that the
Steam language does not define, and the analyzer shows the number in that case.

#### `crates/steamkit-tools/src/nethook/dump.rs` — 134 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/NetHookDump.cs`
**Purpose:** A NetHook dump — the directory of capture files — and the state that is derived from it.
**How it works:** `NetHookDump` holds the `Vec<NetHookItem>` and a `HashMap<i32, Vec<u8>>` of **account auth secrets**. `load_from_directory` lists the directory, keeps the files whose extension is `.bin` (case-insensitive), sorts them by path and loads each one. `add_item_from_file` pushes the item and then calls `capture_account_auth_secret`: if the message is `EMsg::ServiceMethodResponse` and its inner message name is `Credentials.GetAccountAuthSecret#1`, the body is decoded as `CCredentialsGetAccountAuthSecretResponse` and the `secret_id`/`secret` pair is remembered. `account_auth_secret(secret_id)` hands the secret to the remote-play specialization, which needs it to decrypt the payload.
**Notes and gotchas:** The C# tool *watched* the directory for new files and kept the decoded items in
memory; the Rust port reads the directory once, which is what a command-line tool needs. The account
auth secret is a **cross-packet** dependency: a remote-play notification can only be decrypted if the
corresponding `Credentials.GetAccountAuthSecret` response was captured in the same dump, so the
secret has to be collected while the files are loaded and not while they are decoded.

#### `crates/steamkit-tools/src/nethook/message_reading.rs` — 304 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/NetHookItem_MessageReading.cs`
**Purpose:** Reads a captured packet into a header, a body and a payload.
**How it works:** `read_message(bytes)` reads the four-byte little-endian message number, then chooses where the header starts (see the section above), calls `read_header` and then `read_body`. `read_header` returns `Header::{MsgHdrProtoBuf, MsgHdr, ExtendedClientMsgHdr, MsgGCHdr, MsgGCHdrProtoBuf}`; the GC header variants exist because the game coordinator payloads have their own headers. `read_body` dispatches on `MsgUtil::get_msg(raw_emsg)`:
* the four unified-messaging request messages (`ServiceMethod`, `ServiceMethodSendToClient`, `ServiceMethodCallFromClient`, `ServiceMethodCallFromClientNonAuthed`) use the **request** type of the target job name;
* `ServiceMethodResponse` uses the **response** type;
* `ClientServiceMethodLegacy` / `ClientServiceMethodLegacyResponse` decode the `CMsgClientServiceMethodLegacy` wrapper first and then decode the *serialised* method body with the type the method name resolves to;
* everything else goes to `read_message_body`, which tries the protobuf body type first, then the non-protobuf (SteamKit serializable) body type, and finally falls back to the generic protobuf field reader.

`target_job_name(header)` reads `MsgHdrProtoBuf.proto.target_job_name` — the unified-messaging method
name, for example `Credentials.GetAccountAuthSecret#1`. `decode_protobuf_body` decodes with the
registry's decoder and returns the **whole** remaining buffer as the body, so the payload of a
protobuf message is always empty. `inner_message_name(emsg, bytes)` is the C# `ReadInnerMessageName`:
for `ClientToGC`/`ClientFromGC` it decodes the `CMsgGcClient` and looks the game coordinator message
number up in the game's enums, stripping the `k_EMsg` prefix; for the unified-messaging messages it
reads the protobuf header's target job name; for the legacy messages it reads the method name; and
for everything else it returns an empty string. `emsg_to_string_name(raw_emsg)` produces the C# tree
label `EMsg <Name> (<number>)`, with the suffix ` (Non-Protobuf)` appended when the message is not
protobuf backed.
**Notes and gotchas:** The C# used `Lazy<string>` for the target job name so that it was only read
when the body needed it; the Rust port computes it eagerly, which costs a header parse for the
messages that do not need it. The C# `ReadMessageBody` had two further fallbacks — `SteamKit2.Msg<name>`
and `SteamKit2.CMsg<name>` by reflection over the assembly — with `Econ` and `AM` stripped from the
message name; the Rust port resolves those cases through the static `serializable_registry` table
instead.

#### `crates/steamkit-tools/src/nethook/message_type_finder.rs` — 116 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/MessageTypeFinder.cs`
**Purpose:** Resolves the *type* of a message body.
**How it works:** The C# tool did this by **reflection** over the SteamKit2 assembly: `GetProtobufMessageBodyType` looked for the first protobuf type in the `SteamKit2.Internal` namespace whose name *ends with* the name of the message; `GetNonProtobufMessageBodyType` looked for the SteamKit-wire-format message whose message number matches; `GetGCMessageBodyTypeCandidates` did the same for game coordinator messages, by reducing the GC message name to a suffix and looking for every type that ends with it. Rust has no run-time reflection over a set of message types, so the first two lookups became **static tables** (`proto_registry`, `serializable_registry`) that were resolved once, while the port was written, and the third one searches the game coordinator tables by name.

`protobuf_message_body_type(emsg)` consults `message_type_overrides::body_map` first (the explicit override table) and falls back to `proto_registry::protobuf_body_decoder(emsg)`. `non_protobuf_message_body_type(emsg)` delegates to `serializable_registry`. `gc_message_body_type_candidates(raw_emsg, gc_app_id)` returns the override if there is one, otherwise it reduces the GC message name to a suffix with `gc_type_suffix` and collects every type of `proto_registry::BASE_GC_TYPES` **and** of the game's own table whose lower-cased name ends with that suffix; the caller tries the candidates in order and uses the first that decodes.
**Notes and gotchas:** `gc_type_suffix` reproduces the C# reduction
`name.Replace("k_", "").Replace("ESOMsg", "").TrimStart('_').Replace("EMsg", "").TrimStart("GC")`.
The last step is `TrimStart("GC")` in C#, which strips **every** leading `G` and `C` — so
`k_EMsgGCClientHello` becomes `lientHello`, not `ClientHello`. The Rust port reproduces this
exactly (`trim_start_matches(|c| c == 'G' || c == 'C')`), and the suffix is matched
case-insensitively against the type names. The game tables are only defined for TF2, Dota 2, CS2 and
Deadlock (`gc_types_of`).

#### `crates/steamkit-tools/src/nethook/message_type_overrides.rs` — 194 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/MessageTypeOverrides.cs`
**Purpose:** The explicit overrides for the messages whose protobuf type does not follow the naming scheme.
**How it works:** `body_map(emsg)` maps a message number to an explicit `(name, decoder)` pair for twelve messages: `ClientLogonGameServer` → `CMsgClientLogon`; `ClientGamesPlayed`/`ClientGamesPlayedNoDataBlob`/`ClientGamesPlayedWithDataBlob` → `CMsgClientGamesPlayed`; `ClientToGC`/`ClientFromGC` → `CMsgGCClient`; `ClientFriendMsgIncoming`/`ClientFriendMsgEchoToSender` → `CMsgClientFriendMsgIncoming`; `ClientCurrentUIMode` → `CMsgClientUIMode`; `ClientGetNumberOfCurrentPlayersDP` → `CMsgDPGetNumberOfCurrentPlayers`; its response → `CMsgDPGetNumberOfCurrentPlayersResponse`; `AMGameServerUpdate` → `CMsgGameServerData`; `ClientDPUpdateAppJobReport` → `CDataPublisher_ClientUpdateAppJob_Notification`; `ClientPlayingSessionState` → `CMsgClientPlayingSessionState`; `ClientNetworkingCertRequestResponse` → `CMsgClientNetworkingCertReply`. `gc_body_map(app_id, gc_emsg)` does the same for the game coordinator messages, per game: the hello/welcome pairs, the connection-status pair (Dota 2, CS2, Deadlock), the shared-object messages (`k_ESOMsg_Create`/`Destroy`/`Update` → `CMsgSOSingleObject`, `k_ESOMsg_UpdateMultiple` → `CMsgSOMultipleObjects`) and the Dota 2-specific `k_EMsgGCToClientTeamsInfo` / `k_EMsgClientToGCMyTeamInfoRequest`. The GC message numbers are spelled out in the `gc_message` module (4006/4004/4007/4005/4009/4010, 21/23/22/26, 8136/8137).
**Notes and gotchas:** The overrides are consulted **before** the naming-scheme search, exactly as in the
C# — this is why `EMsg::ClientToGC` decodes as `CMsgGCClient` and not as some `…GCClient` type found
by suffix.

#### `crates/steamkit-tools/src/nethook/proto_registry.rs` — 10 621 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/MessageTypeFinder.cs` + `Specializations/UnifiedMessagingHelpers.cs` (the reflection lookups, resolved once).
**Purpose:** The static lookup tables that replace the C# tool's run-time reflection.
**How it works:** The file is **generated** by `crates/steamkit-tools/tools/gen_proto_registry.py` (the header says "do not edit by hand"). It has four parts:

1. A `decoders!` macro that emits one `pub fn decode_<ns>_<Type>(bytes: &[u8]) -> Result<Box<dyn ProtobufBody>, prost::DecodeError>` per protobuf type, with `ProtobufDecoder = fn(&[u8]) -> Result<Box<dyn ProtobufBody>, prost::DecodeError>` as the function-pointer type. The names are `decode_steam_*`, `decode_gc_*`, `decode_csgo_*`, `decode_dota2_*`, `decode_tf2_*`, `decode_deadlock_*` and `decode_webui_*`.
2. `protobuf_body_decoder(emsg) -> Option<(&'static str, ProtobufDecoder)>` — the resolved form of the C# "first protobuf type whose name ends with the message name" search.
3. `service_method_types(service_method_name) -> Option<ServiceMethodTypes>` — the resolved form of the C# `UnifiedMessagingHelpers`, which found the unified-messaging method through the `I<Service>` interface of the assembly and took the parameter or the return type. `ServiceMethodTypes` carries `request_type`/`request_decoder` and `response_type`/`response_decoder`.
4. The game coordinator type tables: `BASE_GC_TYPES`, `TF2_GC_TYPES`, `CSGO_GC_TYPES`, `DOTA2_GC_TYPES` and `DEADLOCK_GC_TYPES`, each a `&[(&'static str, ProtobufDecoder)]`. `BASE_GC_TYPES` is the shared GC namespace; the comment notes that every game compiles its own copy of it, so the copy of the first game is used.

The generator reads `crates/steamkit-protos/proto/**/*.proto` (the canonical Valve message names), the
`prost` output under `target/debug/build/steamkit-protos-*/out/` (the Rust type names) and
`crates/steamkit/src/base/generated/steam_language.rs` (the `EMsg` enum).
**Notes and gotchas:** Because the table is generated from a **build artefact**, it must be
regenerated whenever the protos or the generated `prost` code change; a stale table silently
mis-decodes messages. `BASE_GC_TYPES` being the first game's copy of the shared namespace is a
pragmatic simplification, not a correctness claim.

#### `crates/steamkit-tools/src/nethook/serializable_registry.rs` — 77 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/MessageTypeFinder.cs` (`GetNonProtobufMessageBodyType`).
**Purpose:** The messages of the SteamKit wire format that are **not** protobuf backed.
**How it works:** A `serializable_types!` macro takes a list of `decoder => type => "Name" => EMsg` rows and generates one decoder function per row (`<Type>::read(reader)`) plus the lookup function `non_protobuf_message_body_type(emsg) -> Option<(&'static str, SerializableDecoder)>`. `SerializableDecoder = fn(&mut Reader<'_>) -> Result<Box<dyn SerializableBody>, SteamError>`. The table covers 41 messages: `MsgClientJustStrings`, the three `MsgChannelEncrypt*` messages, `MsgClientLogon`, `MsgClientVACBanStatus`, `MsgClientAppUsageEvent`, `MsgClientUpdateGuestPassesList`, `MsgClientP2PIntroducerMessage`, the eight `MsgClientOGS*` messages, `MsgClientGetFriendsWhoPlayGame` (+ response), the nine `MsgGS*` messages, the eleven `MsgClientChat*` messages, `MsgClientSetIgnoreFriend` (+ response), `MsgClientLoggedOff`, `MsgClientLogOnResponse`, `MsgClientServerUnavailable`, `MsgClientCreateChat` (+ response), `MsgClientMarketingMessageUpdate2` and `MsgClientGetLegacyGameKey` (+ response).
**Notes and gotchas:** The C# found these types by reflection over the SteamKit2 assembly and matched
on the message number; the Rust port points at the **generated** `steam_language::internal` structs of
the `steamkit` crate, which are the same messages the library itself uses. `MsgClientJustStrings` is
registered against `EMsg::Invalid` because the message has no number of its own.

#### `crates/steamkit-tools/src/nethook/emsg_extensions.rs` — 92 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/EMsgExtensions.cs`
**Purpose:** Names the game coordinator messages.
**How it works:** `gc_message_name(raw_emsg, app_id)` masks off the protobuf bit (`0x8000_0000`) and looks the number up in the enums of the game the message belongs to, in a **fixed order**: the game's own message enum first (`EtfgcMsg`, `EdotagcMsg`, `ECsgoGcMsg`, `EgcCitadelClientMessages`), then `EgcBaseMsg`, `EsoMsg`, `EgcItemMsg`, `EgcSystemMsg` and `EgcBaseClientMsg`. The names are the protobuf ones, for example `k_EMsgGCClientHello`. When no enum declares the number, the number itself is returned as a string. `gc_message_number(raw_emsg)` returns `MsgUtil::get_gc_msg(raw_emsg)`.
**Notes and gotchas:** The *order* matters: the C# returned the first enum that declared the number, and
the game-specific enum is consulted first so that a number that is declared in two enums gets the
game-specific name. `EgcSystemMsg` only exists for TF2 and CS2, which is why the two lookup functions
differ in their middle entries.

#### `crates/steamkit-tools/src/nethook/well_known_app_ids.rs` — 13 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/WellKnownAppIDs.cs`
**Purpose:** The app ids of the games the analyzer knows.
**How it works:** Four constants: `COUNTER_STRIKE_2 = 730`, `DOTA_2 = 570`, `TEAM_FORTRESS_2 = 440` and `DEADLOCK = 1_422_450`.
**Notes and gotchas:** Every game-specific table in the crate (`message_type_finder::gc_types_of`,
`emsg_extensions`, `message_type_overrides::gc_body_map`, the shared-object specializations) keys off
these constants; adding a game means adding it here too.

#### `crates/steamkit-tools/src/nethook/protobuf_field_reader.rs` — 155 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/ProtoBufFieldReader.cs`
**Purpose:** The generic protobuf fallback reader, used when the type of a message is not known.
**How it works:** `read_protobuf(bytes) -> NetHookResult<Vec<ProtobufField>>` walks the wire format from offset 0 to the end of the buffer. The tag varint gives the field number (`tag >> 3`) and the wire type (`tag & 7`); the value is read according to the wire type (`Varint`, `Fixed64`, `LengthDelimited`, `Fixed32`, or a group, which is skipped with `skip_group` and recorded as `ProtobufValue::Group`). An unexpected `EndGroup` is a `Malformed` error. The values of a field are **collected in the order they occur** — the Rust counterpart of the C# `Dictionary<int, List<object>>` — so a repeated field keeps every occurrence.
**Notes and gotchas:** The reader reads **until the buffer ends**: a protobuf message of the Steam wire
format is not length-prefixed, so the message reaches to the end of the capture file. A truncated
varint, a length that overflows or a truncated group are `Malformed` errors, not silent truncation.

#### `crates/steamkit-tools/src/nethook/body.rs` — 154 lines
**C# counterpart:** the parts of `NetHookItem_MessageReading.cs` and `ProtoBufFieldReader.cs` that turn bytes into an object the tool can show.
**Purpose:** The model of a decoded message body.
**How it works:** `Body` is a four-variant enum: `Protobuf { type_name, bytes, message }` (a message whose type the analyzer knows), `Fields(Vec<ProtobufField>)` (a protobuf message whose type is unknown), `Serializable { type_name, message }` (a SteamKit-wire-format message that is not protobuf backed) and `Unknown(String)`. `ProtobufBody` and `SerializableBody` are blanket-implemented marker traits (`impl<T: Debug + Send + Sync> ProtobufBody for T`), so any decoded `prost` message can be stored behind `Box<dyn ProtobufBody>`. `ProtobufValue` is the wire-level value (`Varint`, `Fixed64`, `Fixed32`, `Bytes`, `Group`) with a `Display` that renders the bytes as a quoted, lossy UTF-8 string. `WireType::LengthDelimited` is displayed as `String` — the C# tool's label.
**Notes and gotchas:** The C# passed the decoded protobuf message around as an `object` and inspected it
through reflection. Rust has no run-time reflection over a set of message types, so the message is
kept behind the marker trait and the **specializations decode the bytes again** when they need a
concrete type — that is why `Body::decode_as::<T>()` exists and why the `Protobuf` variant keeps the
original bytes. `Body::decode_as` returns `None` for a body that is not a protobuf message or whose
bytes do not decode into `T`.

#### `crates/steamkit-tools/src/nethook/render.rs` — 304 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/TreeNodeObjectExplorer.cs` (the reflection-based property tree).
**Purpose:** Turns a decoded body into the tree of nodes the analyzer shows.
**How it works:** `TreeNode { name, value: Option<String>, children: Vec<TreeNode> }` is `Serialize`, which is what makes `--json` possible. `debug_node(name, debug, show_unset_fields)` and `protobuf_node(type_name, message, show_unset_fields)` take the **`Debug` representation** of the decoded message and split it into fields: `parse_struct` finds the braces, `split_top_level` splits the body on the commas that are not inside brackets or strings, and `find_top_level_colon` finds the `name: value` separator. `field_node` then recurses: an `Option` is unwrapped, a `vec![…]` or `[…]` becomes a list node with `[ i ]` children, and a nested struct becomes a node whose value is the type name. `is_unset(value)` reports `None`, `[]` and `vec![]` as "not set on the wire"; `--show-unset-fields` is what keeps them.
**Notes and gotchas:** This is a **textual** parser over the `Debug` output, not reflection. It is a
deliberate approximation of the C# tree: the field names are the same (they come from the same
`prost` structs) but the values are the `Debug` rendering, so a `Vec<u8>` shows as a list of numbers
rather than as a byte string. `split_top_level` and `find_top_level_colon` both track string literals
and escape sequences, so a comma inside a string does not split a field.

#### `crates/steamkit-tools/src/nethook/tree.rs` — 111 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/NetHookItemTreeBuilder.cs`
**Purpose:** Builds the tree of a captured packet.
**How it works:** `build_tree(item, dump, specializations, show_unset_fields)` reads the capture file, calls `read_message` and creates the root node labelled with `emsg_to_string_name(raw_emsg)`. It then appends three children: `Header` (the `Debug` of the header), `Body` (the body node) and, if the payload is not empty, `Payload` (the upper-case hex of the payload). The specializations then run: a work list is seeded with the message body, every specialization is asked for `read_extra_objects(body, context)`, and the bodies that the specializations return are pushed back onto the work list, so the tree grows until nothing more is found. `write_tree(out, node, depth)` renders the tree as indented text.
**Notes and gotchas:** The recursion is **breadth-first over the work list**, not depth-first, so the
order of the extra objects is the order in which the specializations produced them. A failure is not
propagated: `build_tree` catches the error and returns a single node labelled
`NetHookError encountered whilst parsing item: <error>`, exactly like the C# tool, which showed the
exception in the tree.

#### `crates/steamkit-tools/src/nethook/specializations/mod.rs` — 84 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/ISpecialization.cs`, `IGameCoordinatorSpecialization.cs`
**Purpose:** The trait that the seven specializations implement, and the default set.
**How it works:** `Specialization` has one method, `read_extra_objects(&self, body, context) -> Vec<ExtraObject>`; `GameCoordinatorSpecialization` has `get_extra_objects(&self, body, app_id, context) -> Vec<ExtraObject>`. `ExtraObject { name, body: Option<Body>, node: TreeNode }` — the optional `body` is what allows the tree builder to feed a nested object back into the specializations. `SpecializationContext { dump: &NetHookDump, show_unset_fields: bool }` gives a specialization access to the rest of the dump (for the account auth secret) and to the display flag. `default_specializations()` returns the seven: `ClientServiceMethodSpecialization`, `ClientServiceMethodResponseSpecialization`, `RemoteClientSteamToSteamSpecialization`, `RemoteClientSteamBroadcastSpecialization`, `MultiPacketSpecialization`, `MmsLobbyMessageSpecialization` and `GcGenericSpecialization` (which itself carries the four game shared-object specializations).
**Notes and gotchas:** The C# tool had the same seven (the GC ones were one class per game, the Rust port
folds them into one generic class plus four game-specific shared-object readers). A specialization
that does not recognise the body returns an **empty** vector — it never fails the whole tree.

#### `crates/steamkit-tools/src/nethook/specializations/client_service_method.rs` — 85 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/ClientServiceMethodSpecialization.cs` and `ClientServiceMethodResponseSpecialization.cs`
**Purpose:** Decodes the body of a legacy unified-messaging request or response.
**How it works:** `ClientServiceMethodSpecialization` decodes the body as `CMsgClientServiceMethodLegacy`, takes the `method_name` and the `serialized_method` blob, resolves the method through `proto_registry::service_method_types`, decodes the blob with the request decoder and returns one `ExtraObject` named `Service Method`. `ClientServiceMethodResponseSpecialization` does the same with `CMsgClientServiceMethodLegacyResponse` and the `serialized_method_response` blob, and names the object `Service Method Response`; when the method name is unknown or the blob does not decode it still returns an object — with the value `<null>` — so the tree shows that *something* was there.
**Notes and gotchas:** The *modern* unified-messaging messages (`ServiceMethod`, `ServiceMethodResponse`,
…) are decoded by `message_reading::read_body` itself, not by a specialization; these two only handle
the **legacy** `ClientServiceMethodLegacy` pair.

#### `crates/steamkit-tools/src/nethook/specializations/gc_generic.rs` — 106 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/GCGenericSpecialization.cs`
**Purpose:** Decodes the game coordinator message that a `CMsgGcClient` carries.
**How it works:** The specialization decodes the body as `CMsgGcClient`, takes the `payload`, the `msgtype` and the `appid`, and reads the GC header out of the payload — `MsgGCHdrProtoBuf` for a protobuf GC message, `MsgGCHdr` otherwise (`read_gc_header`). The body that follows the header is decoded by trying every candidate type of `message_type_finder::gc_message_body_type_candidates` in order and using the first that decodes; when none does, the generic protobuf field reader is used (`Body::Fields`). The result is a node named `Game Coordinator Message` with the children `emsg` (the name looked up in the game's enums), `header` and the body node. Afterwards the game-specific specializations are run on the decoded body.
**Notes and gotchas:** The candidate search is *type-name based* (`gc_type_suffix`), so a wrong game or
an unknown message can decode into a plausible but wrong type — this is inherent in the C# design and
is why the overrides table exists. The GC header length is not part of the payload: it is derived from
the header itself.

#### `crates/steamkit-tools/src/nethook/specializations/mms_lobby.rs` — 118 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/MMSLobbyMessageSpecialization.cs`
**Purpose:** Decodes the binary key-value metadata of a matchmaking lobby message.
**How it works:** `lobby_data(body)` finds the metadata blob: `metadata_kv` of `CMsgClientMmsSetLobbyData`, `CMsgClientMmsLobbyData`, `CMsgClientMmsJoinLobbyResponse` and `CMsgClientMmsCreateLobby`, or the `lobby_message` of the two chat messages. The chat messages of CS2 and Dota 2 start with the four-byte engine build version, which is skipped (`chat_data`). `read_binary_key_values` then looks for the Source 2 binary key-value magic `0x564B4256` (`VBKV_MAGIC`); if it is present the cursor jumps to offset 8, and `KeyValue::try_read_as_binary` parses the tree. The result is a node named `Lobby Metadata`.
**Notes and gotchas:** The magic is optional — a blob without it is parsed as a plain key-value tree
starting at offset 0. Only the CS2 and Dota 2 chat messages have the four-byte build-version prefix;
the other games' chat messages are not handled at all.

#### `crates/steamkit-tools/src/nethook/specializations/multi_packet.rs` — 86 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/MultiPacketSpecialization.cs`
**Purpose:** Decodes the messages a multiplexed (`CMsgMulti`) message carries.
**How it works:** The body is decoded as `CMsgMulti`. The `message_body` blob is gzip-decompressed with `flate2::read::GzDecoder` **only when `size_unzipped` is greater than zero** — that is the flag that says the blob is compressed; otherwise it is used as-is. The blob is then a sequence of `[4-byte length][message]` entries; a negative length ends the walk, and the length is clamped to the remaining bytes. Every entry that `read_message` can decode becomes an `ExtraObject` named after the message, with the same `Header` / `Body` / `Payload` children as a top-level packet.
**Notes and gotchas:** The `size_unzipped > 0` test is the only signal for compression — a compressed
blob with `size_unzipped == 0` would be treated as raw. A length that runs past the end of the blob is
clamped rather than rejected, so a truncated multiplex produces a partial message rather than an error.

#### `crates/steamkit-tools/src/nethook/specializations/remote_client_steam.rs` — 231 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/RemoteClientSteamToSteamSpecialization.cs` and `RemoteClientSteamBroadcastSpecialization.cs`
**Purpose:** Decodes the remote-play notifications.
**How it works:** `RemoteClientSteamToSteamSpecialization` decodes `CRemoteClientSteamToSteamNotification`, takes the `secretid` and looks the **account auth secret** up in the dump (`context.dump.account_auth_secret(secret_id)`), decrypts the `encrypted_payload` with `CryptoHelper::symmetric_decrypt` and reads the decrypted bytes as a Steam message. It returns three objects: `Decrypted Header`, `Decrypted Body` and — when there is one — `Decrypted Payload`. `RemoteClientSteamBroadcastSpecialization` decodes `CRemoteClientBroadcastNotification` and parses the payload by hand: a `0xFFFFFFFF` sentinel, the broadcast magic `0xA05F4C21`, a length-prefixed `CMsgRemoteClientBroadcastHeader`, then a length-prefixed body. The body type is derived from the header's `msg_type` by `broadcast_body_type_name` (the `k_ERemoteClientBroadcastMsg…` name with `k_E` → `CMsg` and `BroadcastMsg` → `Broadcast`), and is looked up in the `BROADCAST_BODY_TYPES` table; an unknown type leaves the body as hex.
**Notes and gotchas:** The remote-play decryption depends on a **cross-packet** fact — the auth secret
must have been captured in the same dump, which is why `NetHookDump` collects it while loading. The
five known broadcast body types are `CMsgRemoteClientBroadcastDiscovery`, `…Status`,
`…ClientIdDeconflict`, `…ClientPairingState` and `…ClientPairingExclusivity`; the message numbers
0, 1, 2, 11, 15 and 16 are the ones `broadcast_body_type_name` knows, and 2 and 16 have no registered
decoder (they fall back to hex).

#### `crates/steamkit-tools/src/nethook/specializations/shared_objects.rs` — 384 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Specializations/{CSGO,Dota2,TF2,Deadlock}/*SOHelper.cs` and the `*SOSingleObject*` / `*SOMultipleObjects*` / `*SOCacheSubscribed*` specializations.
**Purpose:** Decodes the shared objects the game coordinator sends.
**How it works:** A `shared_object_decoder!` macro generates, per game, a `decode_<game>(type_id, data) -> Option<(String, Box<dyn ProtobufBody>)>` that maps the shared-object **type id** to a protobuf type: nine CS2 types (1 `CSOEconItem`, 2 `CSOPersonaDataPublic`, 5 `CSOItemRecipe`, 7 `CSOEconGameAccountClient`, 38 `CSOEconItemDropRateBonus`, 40/41 `CSOAccountSeasonalOperation`, 45 `CSOEconCoupon`, 46 `CSOQuestProgress`), eighteen Dota 2 types, seventeen TF2 types and ten Deadlock types. Four `GameCoordinatorSpecialization` implementations then extract the objects from the three carrier messages of their game: `CMsgSoSingleObject` (label `SO`), `CMsgSoMultipleObjects` (labels `New SO` / `Modified SO` / `Removed SO` for Dota 2 and Deadlock, `SO` for the TF2 `objects` field) and `CMsgSoCacheSubscribed` (label `SO`).
**Notes and gotchas:** Each specialization returns early when the app id does not match its game. The
**labels differ per game** because the C# helpers differed: TF2's multiple-objects message has a
single `objects` field, while Dota 2's and Deadlock's have `objects_added` / `objects_modified` /
`objects_removed`. Note that CS2's type 40 and 41 both map to `CSOAccountSeasonalOperation`, and
Dota 2 has both `CSODOTALobby` (2013) and `CSODOTALobby` (2014 → `CSODOTAStaticLobby`).

#### `crates/steamkit-tools/src/bin/steamkit-nethook-analyzer.rs` — 346 lines
**C# counterpart:** `Resources/NetHookAnalyzer2/NetHookAnalyzer2/Program.cs` and `MainForm.cs` (the list and the tree view).
**Purpose:** The `steamkit-nethook-analyzer` command-line front end.
**How it works:** Usage: `steamkit-nethook-analyzer <dump directory> [options]`. The options are `--list` (the default), `--item <sequence>` (the detail of one packet), `--all` (the detail of every packet), `--direction <in|out>`, `--filter <text>`, `--json` and `--show-unset-fields`. The directory is the first non-flag argument. `run` loads the dump, builds the default specializations, computes the inner message name of every item and keeps the ones that pass `matches` — the direction filter and a **case-insensitive substring match** against either the message name (`EMsg …`) or the inner message name. `--list` prints a table (`#`, `Timestamp`, `Dir`, `Message`, `Inner Message`); `--item` and `--all` print the tree (or the JSON of the tree). `format_timestamp` renders the modification time as `YYYY-MM-DD HH:MM:SS` in **UTC**, using a hand-written `civil_from_days` (the Howard Hinnant days-from-civil algorithm) rather than a date library.
**Notes and gotchas:** The C# tool watched the directory live and updated the WPF list; the Rust tool
takes a snapshot. The JSON output serialises the `TreeNode` structure directly (`name`, `value`,
`children`), so `--json` is the machine-readable form of the same tree. `--filter` matches the
**lower-cased** message name and inner name, so it is case-insensitive.

---

#### 06.6 `crates/steamkit-tools/lang/` — the Valve definition files

#### `crates/steamkit-tools/lang/*.steamd` — 7 files, 4 404 lines total
**C# counterpart:** `csharp-original/Resources/SteamLanguage/*.steamd` (byte-identical copies of the same seven files)
**Purpose:** The seven Valve SteamLanguage definition files that drive the enum generator. They are **not** Rust code and they are **not** generated — they are the upstream input, copied into the crate so that the build is self-contained and reproducible offline.
**How it works:** The files are parsed by the entry point `steammsg.steamd`, which `#import`s the others:

| File | Lines | Role |
|---|---|---|
| `steammsg.steamd` | 372 | **Entry point.** The `Msg*` message structs (`MsgClientJustStrings`, `MsgChannelEncrypt*`, `MsgClientLogon`, the `MsgClientChat*` family, the `MsgGS*` family, …) and their `const` protocol versions. Imports `header.steamd` and `gamecoordinator.steamd`. |
| `header.steamd` | 37 | The four message headers: `MsgHdr`, `ExtendedClientMsgHdr`, `MsgHdrProtoBuf`. Imports `emsg.steamd`, `eresult.steamd`, `enums.steamd` and `netheader.steamd`. |
| `emsg.steamd` | 2 027 | The `EMsg` enum — the message numbers of the Steam protocol. This is the file that contains the **duplicate values** the alias mechanism exists for (e.g. `BaseGeneral = 100;` and `GenericReply = 100;`) and many `removed` / `obsolete` markers. |
| `enums.steamd` | 1 754 | The general-purpose enums: `EUniverse`, `EChatEntryType`, `EPersonaState`, `EPersonaStateFlags`, `ELobbyType`, … |
| `eresult.steamd` | 135 | The `EResult` enum — the result codes of every Steam call. |
| `netheader.steamd` | 62 | The UDP layer: `EUdpPacketType<byte>` (an enum **with an explicit underlying type**, so it becomes `#[repr(u8)]`) and the `UdpHeader` / `ChallengeData` structs. |
| `gamecoordinator.steamd` | 17 | The two game coordinator headers: `MsgGCHdrProtoBuf` (with the `protomaskgc` flag) and `MsgGCHdr`. |

**The format.** A `.steamd` file is a sequence of top-level declarations:

```
#import "header.steamd"        ← a preprocessor directive; the path is resolved against --lang-dir

enum NAME { … }                 ← an enum
enum NAME<TYPE> { … }           ← an enum with an explicit underlying type
enum NAME flags { … }           ← an enum whose members are bit flags

class NAME { … }                ← a message struct
class NAME<Enum::Member> { … } ← a message struct bound to a message id
class NAME expects Parent { … } ← the 'expects' clause is parsed and discarded
class NAME removed { … }        ← the struct is not emitted
```

Inside a body a member is one of

```
NAME;                          enum member without a value (auto-increment) / untyped field
TYPE NAME;                     typed member
TYPE NAME = value;             typed member with a default
TYPE<opt> NAME = value;        typed member with a marshal option, e.g. `byte<20> loginKey;`
```

`value` is a `|`-separated list of terms; a term is an integer literal (`0xBAADF00D`, `-700`), the name
of a previously declared member of the same enum, or the name of a member of another enum that has a
single literal value. After the terminating `;` a member may carry a marker:

```
NAME = 5; obsolete              kept, emitted as #[deprecated]
NAME = 6; obsolete "use X"      kept, emitted as #[deprecated = "use X"]
NAME = 7; removed               dropped entirely
NAME = 8; removed "gone"       dropped entirely (the reason is parsed but not emitted)
```

The marshal flags that occur in these files are `const` (31×), `steamidmarshal` (32×), `gameidmarshal`
(4×), `boolmarshal` (5×), `proto` (2×), `protomask` (1×) and `protomaskgc` (1×). Type names map to
Rust primitives as `byte`→`u8`, `short`→`i16`, `ushort`→`u16`, `int`→`i32`, `uint`→`u32`,
`long`→`i64`, `ulong`→`u64`, `char`→`char`, `bool`→`bool`; anything else is a locally declared enum
or class and becomes a generated Rust type.

**The import graph.** `steammsg.steamd` imports `header.steamd` and `gamecoordinator.steamd`;
`header.steamd` imports `emsg.steamd`, `eresult.steamd`, `enums.steamd` and `netheader.steamd`. The
parser analyses an imported file into **its own root** and only then splices its children into the
importing root — exactly like the C# `TokenAnalyzer.Analyze` recursion. The order of the generated
enums therefore follows the *import* order, not the alphabetical order of the files, and an import is
resolved relative to `--lang-dir` (falling back to the path as written).

**Notes and gotchas:** These files are **copies**; the C# originals live in
`csharp-original/Resources/SteamLanguage/` and are the ground truth for the parity check. When Valve
publishes a new revision the copies have to be refreshed and `steamkit-lang-gen` re-run. The
generator is *not* idempotent across a changed import order: because the emitted enums follow the
splice order, re-ordering the `#import` lines re-orders the generated file. The `removed` members do
**not** take part in the C# auto-increment, so a removed member does not shift the values of the
members that follow it.

---

#### 06.7 `Cargo.toml` and `build.rs`

#### `crates/steamkit-tools/Cargo.toml` — 53 lines
**C# counterpart:** `Resources/SteamLanguageParser/SteamLanguageParser.csproj`, `Resources/ProtobufGen/ProtobufGen/ProtobufGen.csproj`, `Resources/ProtobufDumper/ProtobufDumper/ProtobufDumper.csproj`, `Resources/NetHookAnalyzer2/NetHookAnalyzer2/NetHookAnalyzer2.csproj`
**Purpose:** The manifest of the tools crate.
**How it works:** It declares the library (`steamkit_tools`, `src/lib.rs`) and the **four binaries** explicitly, because the binary paths are not the default `src/bin/<name>.rs` for the first one: `steamkit-lang-gen` lives in `src/main.rs`, the other three in `src/bin/`. The dependencies are grouped by purpose: `prost` + `prost-types` (workspace versions) and `prost-build = "0.13"` for the protobuf tooling; `protoc-bin-vendored = "3"` so that the tools work without a system-wide protobuf compiler — the same crate that `crates/steamkit-protos/build.rs` uses; `steamkit` and `steamkit-protos` (path dependencies) so the NetHook analyzer can decode the Steam messages of a capture; `serde` + `serde_json` for the `--json` output; `flate2 = "1"` because the multiplexed messages of a dump are gzip compressed; and `hex` for the payload rendering.
**Notes and gotchas:** The crate **has no `build.rs`** — everything it generates it generates at *run*
time, or the generation is done by the `steamkit-protos` build script (which owns the `OUT_DIR`
pipeline described in [06.3](#063-steamkit-proto-gen--regenerating-the-prost-code)). The `.steamd`
half of the crate deliberately depends on nothing but `std`, which is why `prost` is only pulled in by
the protobuf half. `prost-build` is pinned to `0.13` to match the workspace's `prost` version.

#### `crates/steamkit-tools/build.rs` — none
**C# counterpart:** none — the C# tool projects had no build step either.
**Purpose:** The crate has **no build script**. The `OUT_DIR` code generation that the C# build did at build time is done by `crates/steamkit-protos/build.rs`, not here.
**How it works:** n/a.
**Notes and gotchas:** If a build script is ever added to this crate it must not regenerate
`steam_language.rs` — that file is a **committed artefact** that is checked by
`verify_steam_language.py` against the C# original, and regenerating it on every build would make the
parity check meaningless.

---

#### 06.8 The in-crate verification script

> **Not present in this tree.** The script described below is not included in this snapshot;
> the description is kept for reference. The same caveat applies to `verify_wire_format.py`,
> `check_tree.py` and `audit_mods.py`.

#### `crates/steamkit-tools/verify_steam_language.py` — 284 lines
**C# counterpart:** none — it is a new artefact of the port.
**Purpose:** The parity gate between the C# SteamLanguage and the generated Rust SteamLanguage.
**How it works:** It parses both files and compares them enum by enum. `parse_cs` extracts, for every `public enum` of `SteamLanguage.cs`, the underlying type, whether it carries the `[Flags]` attribute and the `(name, expression)` pairs of its members; the expressions are then evaluated by `eval_cs_expression`, which resolves a `|`-separated expression the way a C# compiler would (a term is a literal, or the already-resolved value of a sibling member). `parse_rust` extracts the enums of the generated file, their `#[repr(..)]` type, their variants, and the alias constants — the `RUST_ALIAS_RE` pattern `pub const (\w+): (\w+) = \2::(\w+);` — and then resolves each alias to the value of the variant it points at, so the resulting member set can be compared one-to-one with the C# one. The comparison then checks, for every enum: the presence of the enum in both files, the presence and the **value** of every member, the **underlying type** (with the C#→Rust mapping `byte`→`u8`, `ushort`→`u16`, `short`→`i16`, `uint`→`u32`, `int`→`i32`, `ulong`→`u64`, `long`→`i64`, and the C# default of `int` → `i32`), and the **flags** flag (detected on the Rust side by the presence of the `pub const fn bits(self)` helper). The script prints the counts of enums and members and the number of differences, and exits non-zero when there is any.
**Notes and gotchas:** The C# reference path defaults to
`SteamKit2/SteamKit2/Base/Generated/SteamLanguage.cs`
and the Rust path to `crates/steamkit/src/base/generated/steam_language.rs`; both can be overridden
with `--cs` / `--rust`. The script is the *authoritative* check on the alias mechanism: without it, the
39 aliases could silently drift away from the C# values. The older copy at `tools/verify_steam_language.py`
does the same job with a slightly different (regex-only) parser.

#### `crates/steamkit-tools/tools/gen_proto_registry.py` — 485 lines
**C# counterpart:** none — it is the generator of the file that replaced the C# reflection.
**Purpose:** Derives `src/nethook/proto_registry.rs` from the protos, the generated `prost` code and the `EMsg` enum.
**How it works:** It reads three sources: `crates/steamkit-protos/proto/**/*.proto` (the canonical Valve message names, extracted with a brace-depth-aware `proto_message_names`), `target/debug/build/steamkit-protos-*/out/**/*.rs` (the Rust type names that `prost` produced — it picks the most recently modified `out` directory), and `crates/steamkit/src/base/generated/steam_language.rs` (the `EMsg` enum). It normalises names with `norm(name)` (lower-cased, non-alphanumerics removed) so that `CMsgGCClient` and `CMsg_GC_Client` compare equal, strips comments from the protos, and parses the `service` / `rpc` declarations with `SERVICE_RE` and `RPC_RE` to build the unified-messaging request/response table. The output is the four-part registry described above: the decoder functions, `protobuf_body_decoder`, `service_method_types` and the five game coordinator type tables.
**Notes and gotchas:** The generator depends on a **build artefact** (`target/debug/build/…/out`), so it
must be run *after* a build of `steamkit-protos`. The module list it knows is
`steam, gc, csgo, dota2, tf2, deadlock, webui`, with `gc` covered by the four game modules.

<!--NEXT-->

#### `crates/steamkit-tools/src/bin/steamkit-depot-dump.rs` - 121 lines
**C# counterpart:** none - it is a new diagnostic for this port, not a port of a C# tool.
**Purpose:** Dumps a Steam depot manifest through the Rust port in a fixed, line-per-item
format, so the output can be compared byte for byte against the reference C# implementation.
**How it works:** Reads the manifest file and the 16-byte depot key from the command line,
calls `DepotManifest::deserialize`, optionally `depot_manifest.decrypt_filenames`, and
prints the manifest summary, every file, every chunk, and - when a chunk file is supplied -
the decrypted chunk. The format is identical to the one `differential-tests/csharp-oracle.ps1`
produces from the C# `SteamKit2.dll`, so the two files can be diffed directly.
**Notes and gotchas:** The chunk is decrypted with `DepotChunk::process`, so this tool
exercises the depot crypto path against real Valve data rather than synthetic fixtures.
## 7. `steamkit-samples` and `steamkit-tests`

Two crates close the port: `steamkit-samples` reproduces the fourteen C# sample
programs under `csharp-original/Samples/`, and `steamkit-tests` reproduces the
xunit suite under `csharp-original/SteamKit2/Tests/`.

The test suite is the main correctness signal of the whole port. It was written
first and compiled last, because the workers were told not to run cargo; when it
was finally compiled and run it exposed **ten real defects** that "it compiles"
had hidden (see [7.3](#73-what-the-suite-found)). Every one of them was a silent
wrong-answer bug, not a compile error. **Run the suite before believing any
claim that a change is complete.** Running the ported *tools* and *samples* then
found two more.

---

### 7.1 `crates/steamkit-samples` - the ported sample programs

#### 7.1.1 Layout

```
crates/steamkit-samples/
  Cargo.toml              15 [[bin]] targets: one per C# sample project, plus 900_live_check
  src/
    lib.rs                8 lines - crate docs + a VERSION constant
    bin/
      000_authentication.rs                    309 lines
      001_authentication_with_qr_code.rs       249 lines
      002_web_cookie.rs                        276 lines
      003_server_list.rs                       169 lines
      004_legacy_login.rs                      121 lines
      005_legacy_steam_guard.rs                252 lines
      010_extending.rs                         298 lines
      011_debug_log.rs                         153 lines
      012_async_jobs.rs                        290 lines
      013_unified_messages.rs                  251 lines
      020_friends.rs                           194 lines
      021_web_api.rs                           124 lines
      022_dota_match_request.rs                415 lines
      900_live_check.rs                        466 lines  (Rust-only)
      999_test_bed.rs                          103 lines
      qr_code/mod.rs                           757 lines  (shared, see 7.1.4)
```

Dependencies: `steamkit`, `steamkit-protos` (the samples use the generated
protobuf messages directly, exactly like the C# samples use the `SteamKit2.Internal`
namespace, and `steamkit` does not re-export them), `tokio`, `tokio-util`,
`anyhow`, `tracing`, `tracing-subscriber`, `serde_json`, and `reqwest` (only so
`021_web_api` can name a `reqwest::Method` for `WebAPI::Interface::call_with_method`).

`src/lib.rs` holds one item: `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`.
No binary currently prints it, so it is dead weight kept for symmetry with the
C# samples' version banners.

#### 7.1.2 Sample index

The *Executed* column is the factual record of what was actually run against the
network during the port. It is not a claim about what the code does.

| Rust binary | C# sample project | Credentials | Executed |
|---|---|---|---|
| `000_authentication.rs` | `Samples/000_Authentication/Program.cs` | username + password | compiled only |
| `001_authentication_with_qr_code.rs` | `Samples/001_AuthenticationWithQrCode/Program.cs` | none (approve in the Steam app) | **run** (2026-10-22) - the QR code was scanned and approved, the logon to a real account succeeded, and the logoff ended the session; see §15.1 |
| `002_web_cookie.rs` | `Samples/002_WebCookie/Program.cs` | username + password | compiled only |
| `003_server_list.rs` | `Samples/003_ServerList/Program.cs` | username + password | **run** - printed the credential prompt, exit 0 |
| `004_legacy_login.rs` | `Samples/004_LegacyLogin/Program.cs` | username + password | compiled only |
| `005_legacy_steam_guard.rs` | `Samples/005_LegacySteamGuard/Program.cs` | username + password | compiled only |
| `010_extending.rs` | `Samples/010_Extending/Program.cs` + `MyHandler.cs` | username + password | compiled only |
| `011_debug_log.rs` | `Samples/011_DebugLog/Program.cs` | username + password | **run** - printed the credential prompt, exit 0 |
| `012_async_jobs.rs` | `Samples/012_AsyncJobs/Program.cs` | username + password | compiled only |
| `013_unified_messages.rs` | `Samples/013_UnifiedMessages/Program.cs` | username + password | compiled only |
| `020_friends.rs` | `Samples/020_Friends/Program.cs` | username + password | compiled only |
| `021_web_api.rs` | `Samples/021_WebAPI/Program.cs` | none | **run** - really reached Steam's Web API and printed the TF2 news |
| `022_dota_match_request.rs` | `Samples/022_DotaMatchRequest/Program.cs` + `DotaClient.cs` | username + password + match id | compiled only |
| `900_live_check.rs` | none (Rust-only) | none (approve in the Steam app) | **run** (2026-10-23) - 9 of 9 checks passed on a real account |
| `999_test_bed.rs` | `Samples/999_TestBed/Program.cs` | none (anonymous logon) | **run** - reached a real Steam CM server, over the websocket transport, back when that transport was the one in use |

**The QR code is also written as an SVG.** The terminal art is drawn with `U+2588 FULL
BLOCK`, which needs a console font that carries the block-drawing glyphs. The sample
therefore also writes the code as an SVG to `%TEMP%\steamkit-qr-login.svg`, which opens in
any browser, where the phone can scan it off the screen. The file is rewritten every time
Steam refreshes the challenge.

Fifteen binaries - the fourteen ports plus the Rust-only `900_live_check`: six executed,
nine compiled but not run.
`001_authentication_with_qr_code` performed the port's first credentialed login. That run
used the sample as it was before the three changes to it described in its entry below. A
second run on 2026-10-23 used the changed sample: it logged on, logged off and exited on
its own. Steam did not move that session to another server before the scan, so the
one-QR-session-at-a-time change has not run live.

**Four of the binaries have changed behaviour since this table was first
written:** `000_authentication.rs`, `001_authentication_with_qr_code.rs`,
`002_web_cookie.rs` and `005_legacy_steam_guard.rs` no longer treat a
`LoggedOnCallback` whose result is `TryAnotherCM` or `ServiceUnavailable` as
fatal. They report it and carry on, because the client is moving to another CM
server and repeating any logon that is waiting for its answer. The details are in 7.1.3.

#### 7.1.3 How the ports differ from the C# projects

**The C# samples at the pinned commit (`94812dc3`) are plain console
applications, not WPF projects.** Every `.csproj` in `csharp-original/Samples/`
declares `<OutputType>Exe</OutputType>` with `<TargetFramework>net10.0</TargetFramework>`,
there is no `UseWPF`, no XAML, no `Window`/`Dispatcher`/`TextBox` anywhere in the
tree, and the samples read their arguments straight out of `args[]`
(`args[0]` = username, `args[1]` = password, `args[2]` = match id for sample 022).
the porting notes §9.2 used to describe them as "WPF projects whose window code only
forwards to the SteamKit2 API"; that characterisation was stale — it described
older SteamKit2 sample revisions, not the tree the port was written against — and
it has been corrected.

What the port *does* keep is the observable contract: the console logic, the
exact console strings (including the legacy `Sample1a` / `Sample3` / `Sample7`
labels, which are a copy-paste artefact of the C# and are reproduced verbatim),
and the argument order. The Rust bins take their arguments from the command
line, exactly like the C# programs do.

**Non-ASCII output is not mangled.** An earlier revision of this guide claimed
that `println!` writes UTF-8 bytes which the Windows console decodes with its OEM
code page. That is not what Rust's std does: since well before the 1.89 MSRV,
`std::io::stdout`/`stderr` convert to UTF-16 and call `WriteConsoleW` whenever
the handle is a console whose output code page is not 65001, so the code page is
not involved. The C# is the side that encodes into the console code page
(`Console.OutputEncoding`, OEM by default) and writes bytes; both render the
block characters. The real differences are that redirected output is UTF-8 in
Rust and OEM-encoded in C#, and that a raw non-UTF-8 byte write to a console
fails in Rust. See the porting notes §17.2.

**Async shape.** The C# samples drive a `CallbackManager` and call
`RunWaitCallbacks`; the Rust ports subscribe directly on the `SteamClient` and
drive `run_wait_all_callbacks(Duration::from_secs(1))` in a loop, with an
`AtomicBool` as the "is running" flag.

**A failed logon is not always the end of the session.** Four samples - `000`,
`001`, `002` and `005` - handle the `LoggedOnCallback` differently from the
C#: a result of `EResult::TryAnotherCM` or `EResult::ServiceUnavailable` is
**reported and then ignored**, while every other non-`OK` result is still
fatal. The reason is in the client, not in the sample: the Rust port moves to a
different CM server by itself when the server says so (see the `cm_client.rs`
entry and [§3.2.1](#321-the-one-path-that-is-not-in-that-diagram-the-cm-server-switch)),
and it repeats the logon on the new server. The C# marks the server bad and
leaves the reconnection to the caller, so the C# samples give up - and the
ported samples would have given up too, had they not been changed to match the
new client behaviour. The message the samples print is:

```
The CM server asked for a different one; the client moves on its own.
```

The message used to end "the logon is repeated", which is wrong when no logon has been
sent yet: the live run of sample `001` showed that Steam also sends this answer to a
session that has not logged on (§15.3).

`003_server_list.rs`, `004_legacy_login.rs`, `010_extending.rs`,
`011_debug_log.rs`, `012_async_jobs.rs`, `013_unified_messages.rs`,
`020_friends.rs`, `022_dota_match_request.rs` and `999_test_bed.rs` do not
subscribe to `LoggedOnCallback` at all, or do not log on, so they need no
change.

#### 7.1.4 `mod qr_code` - the shared QR renderer

`crates/steamkit-samples/src/bin/qr_code/mod.rs` (757 lines) is the only shared
module in the samples crate. It is declared with `mod qr_code;` inside
`001_authentication_with_qr_code.rs`, and because it lives in a sub-directory of
`bin/`, cargo does **not** treat it as a second binary.

The C# sample draws the challenge URL with the third-party `QRCoder` package
(`QRCodeGenerator.CreateQrCode(url, ECCLevel.L)` + `AsciiQRCode.GetGraphic(1, drawQuietZones: false)`).
The samples crate may not take a third-party dependency for that, so this module
implements the slice of ISO/IEC 18004 that is needed:

* byte mode, error-correction level `L`, versions 1 to 10;
* a `VERSIONS` table of ten `VersionInfo` records (total codewords, EC codewords
  per block, block counts and data-codeword counts for the two block groups);
* `ALIGNMENT_POSITIONS` for the alignment patterns and `REMAINDER_BITS` for the
  trailing remainder bits;
* the eight data masks, with the four penalty weights `N1`/`N2`/`N3`/`N4`
  (3 / 3 / 40 / 10) used to pick the least-penalised mask;
* Reed-Solomon error correction over GF(256) (the `0x1D` reduction polynomial),
  block interleaving, format information (BCH, masked with `0x5412`) and version
  information (BCH, for versions 7 and up).

Public API:

* `QrCode::encode(text: &str) -> Result<QrCode, String>` - picks the smallest
  version that fits (`choose_version`), encodes, places the codewords, chooses
  and applies the mask, and draws the format/version information. Returns
  `Err` with a message when the text is too long for version 10.
* `QrCode::to_ascii_art(&self) -> String` - renders the module matrix.

**Gotcha:** despite the name, `to_ascii_art` is not pure ASCII. A dark module is
drawn as two `U+2588 FULL BLOCK` characters and a light module as two spaces, so
that the art keeps the square shape of the code. The glyphs need a console font
that carries them; the SVG has no such dependency (see 7.1.3).

#### 7.1.5 The binaries

##### `crates/steamkit-samples/src/bin/000_authentication.rs` - 291 lines
**C# counterpart:** `Samples/000_Authentication/Program.cs`
**Purpose:** The modern authentication flow - exchange credentials for an access
token through the `Authentication` unified service, then log on to the Steam
network with the resulting refresh token.
**How it works:** `main` reads `args[0]`/`args[1]` as username and password; if
either is missing it writes `Sample1a: No username and password specified!` to
**stderr** and returns `Ok(())` (exit 0), matching the C# `Console.Error.WriteLine`.
It builds a `SteamClient`, fetches the `SteamUser` handler, and subscribes to
`ConnectedCallback`, `DisconnectedCallback`, `LoggedOnCallback` and
`LoggedOffCallback`. On connect it spawns `on_connected`, which fills an
`AuthSessionDetails` (username, password, `is_persistent_session: false`, the
stored guard data, and a `UserConsoleAuthenticator`), calls
`begin_auth_session_via_credentials`, then
`polling_wait_for_result(CancellationToken::new())`. The refresh token from the
poll response goes into `LogOnDetails.access_token`. A hand-written base64url
decoder then parses the payload segment of both the access token and the
refresh token and prints them as pretty JSON. The callback loop runs
`run_wait_all_callbacks(Duration::from_secs(1))` until an `AtomicBool` is
cleared; on a successful logon the sample logs off again.
**Notes and gotchas:** needs real credentials, and SteamGuard will block it
unless the account is not 2FA-protected. The C# keeps the guard data in a local
variable captured by the callback closure; the Rust closures are `Fn`, so the
value lives in an `Arc<Mutex<Option<String>>>` with poison-tolerant
`read_guard_data` / `write_guard_data` helpers. The sample also demonstrates
that a *refresh* token, not the access token, is what `LogOnDetails` wants.
**A `LoggedOnCallback` with `TryAnotherCM` or `ServiceUnavailable` is reported
and then ignored**, not treated as a failed logon: the client is moving to
another CM server by itself and will repeat the logon there. Every other
non-`OK` result is still fatal.

##### `crates/steamkit-samples/src/bin/001_authentication_with_qr_code.rs` - 231 lines
**C# counterpart:** `Samples/001_AuthenticationWithQrCode/Program.cs`
**Purpose:** The QR-code authentication flow - Steam hands out a challenge URL,
the user confirms the login from the Steam mobile app.
**How it works:** takes **no arguments**. On `ConnectedCallback` it calls
`begin_auth_session_via_qr(&AuthSessionDetails::new())`, then draws the
challenge URL immediately and spawns a task that redraws it every time the
`challenge_url_changed()` receiver yields a new URL ("Steam has refreshed the
challenge url"). `draw_qr_code` prints the URL and then the ASCII art from
`mod qr_code`; if the URL is too long it prints "The challenge URL is too long
to be rendered as a QR code." and returns. After
`polling_wait_for_result` it logs on with the refresh token and, like sample
000, logs off again once the logon succeeds. `DisconnectedCallback` ends the
program, as in the C#.
**Notes and gotchas:** the only sample that does not need credentials, and the
only one that cannot run unattended - the user has to scan the code. The C#
draws the code with the `QRCoder` NuGet package (version 1.8.0); the Rust port
replaces it with the dependency-free `mod qr_code` described in 7.1.4. The
challenge URL contains a `|`-separated token, so it is long; version 1-10 at
error-correction level `L` is enough for it in practice. **A `LoggedOnCallback`
with `TryAnotherCM` or `ServiceUnavailable` is reported and then ignored**,
because the client moves to another CM server by itself.
**One QR session at a time.** A move to another CM server raises `ConnectedCallback`
again, and Steam moves an idle, unauthenticated session every minute or two (§15.3). The
C# starts a QR session per `Connected`; done here, every move left the previous session
polling Steam in the background - four were running by the time the live run's code was
scanned. The `QrLogon` struct holds a `CancellationToken` for the session in progress: a
new `Connected` cancels it (which ends its polling and its refresh task) before starting
the next, and once the logon has been sent no new session is started, because the client
repeats the logon on a new server by itself.
**It stops on `DisconnectedCallback` again.** An earlier version kept running after a
disconnection, on the assumption that the client would come back on another server. The
client no longer announces a move as a disconnection, so a `DisconnectedCallback` is the
end of the session - and without stopping, the sample never exited after its logoff.

##### `crates/steamkit-samples/src/bin/002_web_cookie.rs` - 260 lines
**C# counterpart:** `Samples/002_WebCookie/Program.cs`
**Purpose:** Shows how the access token of an authentication session is used to
build the `steamLoginSecure` cookie, and how to renew it.
**How it works:** takes username and password (missing arguments produce the
same `Sample1a:` message on stderr, a copy-paste artefact of the C#). It
authenticates exactly like sample 000, then stores the access token and the
refresh token in two `Arc<Mutex<String>>` slots. On `LoggedOnCallback` it spawns
`on_logged_on`, which builds `format!("{}||{}", steam_id.render(false), access_token)`
- the exact shape of the `steamLoginSecure` cookie - and then calls
`generate_access_token_for_app(steam_id, refresh_token, false)` to renew it,
storing the new access token and, if Steam returned a new one, the new refresh
token.
**Notes and gotchas:** the cookie string is assigned to `_steam_login_secure` and
never used; the C# leaves the actual cookie usage to the reader as well. The
sample does not print the cookie. The 24-hour expiry of the access token and the
`allowRenewal` flag are described in the comments only - there is no renewal
timer in the sample. **A `LoggedOnCallback` with `TryAnotherCM` or
`ServiceUnavailable` is reported and then ignored**, because the client moves
to another CM server and repeats the logon.

##### `crates/steamkit-samples/src/bin/003_server_list.rs` - 162 lines
**C# counterpart:** `Samples/003_ServerList/Program.cs`
**Purpose:** Uses the persisted server list and the persisted cell id to improve
the chance of a successful connection.
**How it works:** takes username and password; the missing-argument message is
`Sample7: No username and password specified!` on **stdout** (`println!`, not
`eprintln!` - the C# uses `Console.WriteLine` here, and the port keeps that
difference). It then reads `cellid.txt` from the working directory: a
successful `u32` parse prints "Using persisted cell ID {n}", a parse failure
prints "Error parsing cellid from cellid.txt. Continuing with cellid 0." and
falls back to 0. The configuration is built with
`SteamConfiguration::create(|b| { b.with_cell_id(cell_id);
b.with_server_list_provider(Arc::new(FileStorageServerListProvider::new("servers_list.bin"))) })`.
After a successful logon it writes the new cell id back to `cellid.txt` from a
spawned task and prints "Successfully logged on! Press Ctrl+C to log off...". A
second spawned task waits on `tokio::signal::ctrl_c()` and then logs off.
**Notes and gotchas:** needs credentials. Writes two files into the current
working directory (`cellid.txt`, `servers_list.bin`). Unlike the other samples
it does **not** log off on success - it stays connected until Ctrl+C. The
`AccountLogonDenied` branch prints "This account is SteamGuard protected."

##### `crates/steamkit-samples/src/bin/004_legacy_login.rs` - 118 lines
**C# counterpart:** `Samples/004_LegacyLogin/Program.cs`
**Purpose:** The old login flow - the username and password go straight to the
Steam network.
**How it works:** takes username and password (`Sample1: No username and
password specified!` on stdout). On `ConnectedCallback` it calls
`steam_user.log_on(&LogOnDetails { username, password, .. })` with no access
token. On `LoggedOnCallback` it prints "Successfully logged on!" and logs off
again.
**Notes and gotchas:** the flow only works for simple cases where the password
does not have to be remembered; the comments point at sample 000 as the
preferred flow. The `AccountLogonDenied` branch prints "This account is
SteamGuard protected." and refers the reader to sample 005.

##### `crates/steamkit-samples/src/bin/005_legacy_steam_guard.rs` - 235 lines
**C# counterpart:** `Samples/005_LegacySteamGuard/Program.cs`
**Purpose:** The old login flow with SteamGuard - the server answers the first
logon attempt with `AccountLogonDenied` and mails an auth code; the second
attempt has to carry that code.
**How it works:** takes username and password (`Sample5: No username and
password specified!` on stdout). A `GuardCodes { auth_code, two_factor_code }`
struct lives in an `Arc<Mutex<...>>`. The first `log_on` sends both codes as
`None`. On `LoggedOnCallback` it distinguishes
`EResult::AccountLogonDenied` (email auth code) from
`EResult::AccountLoginDeniedNeedTwoFactor` (authenticator app code) and spawns
`ask_for_auth_code` / `ask_for_two_factor_code`, which read a line from a
`tokio::io::BufReader<Stdin>` behind a `tokio::sync::Mutex`. Because Steam
disconnects after `AccountLogonDenied`, the `DisconnectedCallback` handler
prints "Disconnected from Steam, reconnecting in 5..." and reconnects after a
five-second sleep; the reconnect re-enters the `ConnectedCallback` handler,
which now sends the code that was collected.
**Notes and gotchas:** needs credentials on a SteamGuard-protected account. The
email prompt prints the *domain* the code was sent to
(`callback.email_domain()`), exactly as the C# does. The sample never logs off
on success - it stays connected. The `read_line` helper returns `None` at EOF
and trims the input. **A `LoggedOnCallback` with `TryAnotherCM` or
`ServiceUnavailable` is reported and then ignored**, because the client moves
to another CM server and repeats the logon - which matters here, because the
sample already reconnects by itself after Steam's `AccountLogonDenied`
disconnect.

##### `crates/steamkit-samples/src/bin/010_extending.rs` - 289 lines
**C# counterpart:** `Samples/010_Extending/Program.cs` and `Samples/010_Extending/MyHandler.cs`
**Purpose:** Extending SteamKit2 with a custom message handler and a custom
callback.
**How it works:** the `mod my_handler` submodule is the Rust counterpart of
`MyHandler.cs` and contains a `MyCallback` (implementing `CallbackMsg`, carrying
an `EResult`) and a `MyHandler` (implementing `ClientMsgHandler`). The handler
keeps a `Weak<SteamClient>` back-reference; `handle_msg` matches
`EMsg::ClientLogOnResponse`, decodes the body with
`ClientMsgProtobuf::<CMsgClientLogonResponse>::from_packet`, converts the
`eresult` field to an `EResult`, prints "HandleLogonResponse: {result:?}" and
posts the custom callback with `client.post_callback(Box::new(MyCallback::new(result)))`.
`main` registers the handler with `steam_client.add_handler(MyHandler::new(&steam_client))`,
retrieves it with `steam_client.get_handler::<MyHandler>()`, and subscribes to
`MyCallback` to print "OnMyCallback: {:?}". The handler also exposes a
`do_something()` that writes to the debug log, and a `log_off()` helper marked
`#[allow(dead_code)]` because it illustrates how a handler sends messages.
**Notes and gotchas:** needs credentials. The `ClientMsgHandler` trait is
declared with `#[async_trait]` in the library, but the samples crate does not
depend on `async-trait`, so the desugared signature is written out by hand as a
type alias `HandlerFuture<'a>` with the three lifetime parameters and the
`where` clause - that is the part of this file that looks strange and is
deliberate. The C# `GlobalSuppressions.cs` exists only to silence an
IDE0060 warning on the unused `LogOff` parameters.

##### `crates/steamkit-samples/src/bin/011_debug_log.rs` - 150 lines
**C# counterpart:** `Samples/011_DebugLog/Program.cs`
**Purpose:** How to listen to the SteamKit2 debug log.
**How it works:** before it even looks at the arguments the sample installs a
listener and switches the log on:
`DebugLog::add_listener(Arc::new(MyListener))` and `DebugLog::set_enabled(true)`.
`MyListener` implements `IDebugListener` and prints
`MyListener - {category}: {msg}`. The commented-out
`DebugLog::add_listener_function(|category, msg| ...)` line is the C# sample's
commented-out anonymous-method listener, kept for the same reason. It then takes
username and password (`Sample3: No username and password specified!` on stdout)
and logs on with the legacy username/password flow, exactly like sample 004.
**Notes and gotchas:** needs credentials. The debug log is a process-wide
global, and enabling it makes the console output very verbose - that is the
point of the sample. A commented-out block shows how to attach
`NetHookNetworkListener` to dump raw packets for NetHookAnalyzer, with the
C#'s own warning that the dumps contain the Steam password. Because the
listener is installed *before* the argument check, a run without credentials
still emits debug-log lines before the prompt.

##### `crates/steamkit-samples/src/bin/012_async_jobs.rs` - 284 lines
**C# counterpart:** `Samples/012_AsyncJobs/Program.cs`
**Purpose:** Job-based messages used asynchronously, through the `SteamApps`
handler.
**How it works:** takes username and password (`Sample9: No username and
password specified!` on stdout). It fetches the `SteamApps` handler and, on
`LoggedOnCallback`, spawns `on_logged_on`, which for app 730 (CS:GO) and depot
731:
1. awaits `steam_apps.get_depot_decryption_key(depot_id, app_id)` and prints the
   key as `XX-XX-...` (a hand-written `to_hex_string` that reproduces
   `BitConverter.ToString`);
2. awaits `steam_apps.pics_get_access_tokens(Some(app_id), None)` and reads the
   token for the app out of the `app_tokens` map, defaulting to 0;
3. builds a `PICSRequest::new(app_id, access_token)` and awaits
   `pics_get_product_info(Some(&request), None, false)`, then inspects
   `result_set.complete` / `result_set.failed` and, in the failure and timeout
   cases, looks the app up with `find_product_info` to see whether Steam sent
   the data before failing;
4. spawns one more task that awaits a second depot-key request and then clears
   the running flag, which ends the callback loop.
**Notes and gotchas:** needs credentials and an account that owns (or can see)
CS:GO. The sample deliberately does **not** subscribe to the `SteamApps`
callbacks - it uses the async job API directly, while the callbacks keep
working alongside it. The C# comment about "older VS/compiler, access the
underlying task" has no Rust analogue, so the last step is simply a second
`tokio::spawn`.

##### `crates/steamkit-samples/src/bin/013_unified_messages.rs` - 261 lines
**C# counterpart:** `Samples/013_UnifiedMessages/Program.cs`
**Purpose:** The unified service API, reached over the Steam network instead of
over HTTP.
**How it works:** takes username and password (`Sample8: No username and
password specified!` on stdout). Before connecting it subscribes to
`CGameNotificationsOnNotificationsRequestedNotification` through
`steam_client.callback_manager().subscribe_service_notification`, which creates
the `GameNotificationsClient` service and registers the decoder on the way,
printing the Steam ID and app id of a user who started a game. On logon it builds a
`CPlayerGetGameBadgeLevelsRequest { appid: Some(440) }` (TF2), creates the
`Player` service with `steam_unified_messages.create_service("Player")`, sets the
response type to `CPlayerGetGameBadgeLevelsResponse`, and sends the request. It
then sends the *same* request a second time directly through
`steam_unified_messages.send_message::<Req, Resp>("Player.GetGameBadgeLevels#1", request)`
to show the manual `<Service>.<Method>#<Version>` name format. It prints the
player level and each badge series/level, sleeps ten seconds so that possible
notifications arrive, and logs off.
**Notes and gotchas:** needs credentials. The C# sample used reflection to call
the service method; the Rust port uses the generated protobuf types from
`steamkit-protos` directly, which is the reason the samples crate depends on
that crate.

##### `crates/steamkit-samples/src/bin/020_friends.rs` - 191 lines
**C# counterpart:** `Samples/020_Friends/Program.cs`
**Purpose:** Basic interaction with the client's persona state and friends list.
**How it works:** takes username and password (`Sample4: No username and
password specified!` on stdout). The configuration is built with
`builder.with_protocol_types(ProtocolTypes::TCP)` - it deliberately pins TCP
rather than the default `TCP | WebSocket`. It fetches the `SteamFriends`
handler and subscribes to `AccountInfoCallback` (on which it calls
`set_persona_state(EPersonaState::Online)`), `FriendsListCallback` (prints
"We have {n} friends", then each friend's `render(false)` Steam3 id, and
auto-accepts pending invites by calling `add_friend_by_steam_id` for every
friend whose relationship is `EFriendRelationship::RequestRecipient`),
`FriendAddedCallback` (prints "{name} is now a friend") and
`PersonaStateCallback` (prints "State change: {name}").
**Notes and gotchas:** needs credentials. The sample never logs off - it waits
for more persona state changes until the connection drops. The account info
callback must arrive before friends can be interacted with; the C# comment
explains that it is posted shortly after a successful logon.

##### `crates/steamkit-samples/src/bin/021_web_api.rs` - 103 lines
**C# counterpart:** `Samples/021_WebAPI/Program.cs`
**Purpose:** Using the WebAPI utilities against the Steam Web APIs. This is
the sample that needs no credentials and the only one that was actually run
against a live Steam service.
**How it works:** it takes **no arguments**. It acquires the interface with
`WebAPI::get_interface("ISteamNews", "")` and calls
`call("GetNewsForApp", 1, &web_api_args([("appid", WebApiValue::from(440u32))]))`
for TF2, printing each news title with `print_news`. It then calls version 2 for
app 570 (Dota 2), where the response shape differs (`newsitems` is a single
node, not a `newsitem` list), and version 2 for app 730 with
`maxlength=100, count=5`, wrapped in a `match` that prints
"Unable to make GetNewsForApp API request: {error}". It then shows the
API-key form: `WebAPI::get_interface("ISteamUserAuth", "APIKEYGOESHERE")`,
`set_timeout(Duration::from_secs(5))`, and
`call_with_method(reqwest::Method::POST, "AuthenticateUser", 1, ...)` whose
failure is caught and printed. The last block re-acquires the interface as an
explicit `Interface` value and calls it with
`web_api_args([("appid", WebApiValue::Text("440".to_string()))])` to show the
non-dynamic call style.
**Notes and gotchas:** no credentials. The `APIKEYGOESHERE` call is expected to
fail - that is the point of the `match`. The C# uses C#'s `dynamic` dispatch
(`steamNews.GetNewsForApp2(appid: 570)`); Rust has no dynamic dispatch, so the
port calls the interface explicitly with a version number. The responses are
`KeyValue` trees, and the sample prints the TF2 news **with `?` instead of the
real characters** - the console-encoding deviation of 7.1.3, in the one sample
where it is actually visible.

##### `crates/steamkit-samples/src/bin/022_dota_match_request.rs` - 410 lines
**C# counterpart:** `Samples/022_DotaMatchRequest/Program.cs` and `Samples/022_DotaMatchRequest/DotaClient.cs`
**Purpose:** Requests the details of a Dota 2 match from the Dota game
coordinator.
**How it works:** takes **three** arguments: username, password and match id.
Missing arguments print "Usage:" / "DotaMatchRequest <username> <password>
<matchid>"; a match id that does not parse prints "Invalid Match ID!". The
`mod dota_client` submodule is the Rust counterpart of `DotaClient.cs` and owns
the whole flow: it builds a `SteamClient`, fetches the `SteamUser` and
`SteamGameCoordinator` handlers, and on `ConnectedCallback` logs on. On
`LoggedOnCallback` it sends a `CMsgClientGamesPlayed` message with
`GameID::from_app_id(570)` (SteamKit exposes no handler for "play game", so the
message is sent by hand), waits five seconds, and sends a
`ClientGCMsgProtobuf::<CMsgClientHello>` with
`engine = ESourceEngine::KEseSource2` to the GC for app 570. It then
dispatches on the `MessageCallback` message number: `KEMsgGcClientWelcome`
prints the GC version and sends a `CMsgGcMatchDetailsRequest` with the match
id; `KEMsgGcMatchDetailsResponse` stores the `CMsgDotaMatch` in an
`Arc<Mutex<Option<...>>>>`, sets the `got_match` flag and disconnects. `wait()`
loops `run_wait_all_callbacks(Duration::from_secs(1))` until that flag is set.
`print_match_details` prints roughly thirty fields of the match explicitly.
**Notes and gotchas:** needs credentials *and* a real match id. The C# prints
the match with reflection over the protobuf message; the Rust port prints the
fields explicitly, because the generated Rust structs have no reflection. The
match id is a `u64` in both.

##### `crates/steamkit-samples/src/bin/900_live_check.rs` - 471 lines
**C# counterpart:** none - a Rust-only check, not a port of a C# sample.
**Purpose:** Checks against live Steam what a logged-on session can do, with a single QR
scan and no password.
**How it works:** takes **no arguments**. It logs on with the QR flow of sample `001`
(including its `QrLogon` one-session-at-a-time logic), then runs nine checks, each
printing a `PASS` or `FAIL` line: the logon; the SteamID (assigned, an individual
account); the `AccountInfoCallback`; the first full `FriendsListCallback`; the unified
message `Player.GetGameBadgeLevels#1`; the PICS access token and product info of app 730
(Counter-Strike 2, free, so available to every account); the depot key of its depot 731;
that the session stays up for 60 seconds while idle; and that `log_off` ends the session
and it is not resumed. The callbacks only record what they saw in an `Observed` struct;
the checks run in order in `main` and poll it through `wait_for`, with a 30-second limit
per check and ten minutes for the scan. It exits with status 1 if any check failed.
**Notes and gotchas:** it runs on the account of whoever scans the code, so it is
written to change nothing there: it never sets the persona state - the account does not
appear online to friends - and it does not accept friend requests (sample `020` does).
It prints no token, no key and no SteamID, only counts and results. It needs a person
with the Steam mobile app, so it cannot run in CI. The first run (2026-10-23) passed all
nine checks (§15.1).

##### `crates/steamkit-samples/src/bin/999_test_bed.rs` - 101 lines
**C# counterpart:** `Samples/999_TestBed/Program.cs`
**Purpose:** A test bed to nethook various packets for later use in tests.
**How it works:** takes **no arguments**. It attaches a
`NetHookNetworkListener` with `steam_client.set_debug_network_listener(...)`,
fetches the `SteamUser` and `SteamApps` handlers, and on `ConnectedCallback`
calls `steam_user.log_on_anonymous()`. On `LoggedOnCallback` it requests the
product info of app 480 (Spacewar) twice - once with `pics_get_product_info(
Some(&request), None, false)` and once with `true` as the last argument - and
then logs off. The main loop is
`while !cancellation.is_cancelled() { match steam_client.wait_for_callback_async(cancellation.clone()).await { Ok(callback) => steam_client.callback_manager().dispatch(callback), Err(_) => break } }`,
so it dispatches callbacks one at a time instead of using
`run_wait_all_callbacks`.
**Notes and gotchas:** no credentials - it logs on **anonymously**. This is the
sample that reached a real Steam CM server. The recorded exchange (from
§12 of this guide) is:

```
out  ClientHello            protocol_version = 65581
out  ClientLogon            language = "english", cell_id = 0, machine_id = BB3/FF2/3B3
in   ClientServerUnavailable
```

`65581` is `MsgClientLogon.CurrentProtocol` - the same value the C# sends. The
server's `ClientServerUnavailable` is a server-side policy decision (Steam
frequently refuses anonymous logons from datacentre IP ranges); the C# handles
`EMsg.ClientServerUnavailable` exactly the same way, by logging and
disconnecting. The whole exchange was decoded with the port's own
`steamkit-nethook-analyzer`, which is itself the proof that the analyzer works.
The nethook dump files that the listener writes contain the machine id.

**That capture is a record of one run, not a statement about the transports
today.** It was taken when the websocket transport had just been wired up, which
is why it is the websocket that appears in it. The later, controlled comparison
against the C# reference — the one that decides which transport the port uses —
found that the server does not answer the websocket's `ClientHello` at all, and
that the TCP transport is the one that completes the flow. See
[§15](#15-what-has-been-verified-against-live-steam).

---

### 7.2 `crates/steamkit-tests` - the ported xunit suite

#### 7.2.1 Layout

```
crates/steamkit-tests/
  Cargo.toml
  src/
    lib.rs                          1 line - placeholder so the manifest resolves
  tests/
    common/mod.rs                  481 lines - shared fixtures
    adler32.rs                     299 lines    27 tests
    async_job.rs                   661 lines    22 tests
    callback_manager.rs            640 lines    19 tests
    cdn_client.rs                  212 lines    14 tests (2 ignored)
    client_msg.rs                    105 lines     2 tests
    cm_client.rs                   219 lines     5 tests
    crypto_helper.rs                49 lines     1 test
    date_utils.rs                   71 lines     1 test
    debug_log.rs                   219 lines     9 tests
    depot_chunk.rs                 557 lines    15 tests
    depot_e2e.rs                   297 lines     1 test (1 ignored)
    depot_manifest.rs              225 lines     5 tests
    file_storage_server_list_provider.rs  83 lines  1 test
    game_id.rs                      55 lines     3 tests
    http_client_purpose.rs          97 lines     4 tests
    isolated_storage_server_list_provider.rs 68 lines 1 test
    key_value.rs                   793 lines    33 tests
    lzma.rs                        274 lines     4 tests
    machine_info.rs                327 lines     8 tests
    net_helpers.rs                 152 lines     6 tests
    network_listener.rs             82 lines     1 test
    packet.rs                      139 lines     1 test
    server_record.rs               188 lines     7 tests
    simple_console_debug_listener.rs 71 lines   2 tests (1 ignored)
    smart_cm_server_list.rs        444 lines    11 tests
    steam_client.rs                211 lines    12 tests
    steam_configuration.rs         382 lines    23 tests
    steam_game_server.rs            66 lines     2 tests
    steam_id.rs                    865 lines    62 tests
    steam_user.rs                  142 lines     8 tests
    stream_helpers.rs              182 lines     2 tests
    web_api.rs                     488 lines    12 tests
    websocket_uri.rs                53 lines     4 tests
    data/                          12 binary fixtures + .gitattributes
```

`crates/steamkit-tests/src/lib.rs` is a one-line placeholder ("Placeholder lib
target so the workspace manifest resolves while the test suite is being
written."). The crate has no library code; everything lives in the integration
test binaries.

**34 files** in `tests/` (33 test files plus `common/mod.rs`) and **328 test
functions**: 262 plain `#[test]` plus 66 `#[tokio::test]` (63 of them plain, 2
with `flavor = "multi_thread", worker_threads = 2` and 1 with
`flavor = "multi_thread"`).

Each file directly under `tests/` is compiled as its own crate and its own test
binary. That is why the shared helpers live in the `common/` sub-directory
(another sub-directory, so it is not a test binary itself) and are pulled in
with `mod common;`.

Dependencies: `steamkit`, `steamkit-protos`, `steamkit-tools`, plus `tokio`,
`tokio-util`, `futures`, `async-trait`, `anyhow`, `thiserror`, `tracing`,
`serde_json`, `hex`, `base64`, `rand`, `reqwest` (with the `blocking` feature)
and `prost`/`prost-types`. Dev-dependencies add `tokio` with `test-util`,
`macros` and `rt-multi-thread`.

#### 7.2.2 `tests/common/mod.rs` - the shared fixtures

`tests/common/mod.rs` (481 lines) is the Rust counterpart of the C#
`SteamKit2/Tests/Helpers/` folder **plus** the collection definition in
`Tests/NotThreadSafeResourceCollection.cs`. It provides:

* `data_dir() -> PathBuf` - `<crate>/tests/data`, the directory that holds the
  binary assets the C# embeds as `Tests.Files.*` manifest resources.
* `data_file(name) -> PathBuf` and `read_data_file(name) -> Vec<u8>` - the path
  of an asset and its contents; `read_data_file` panics with the path and the
  OS error if the file cannot be read.
* `connected_steam_client() -> Arc<SteamClient>` - the counterpart of
  `Tests/Helpers/ConnectedSteamClient.cs`. The C# helper creates a `SteamClient`
  and forces its `IsConnected` flag to `true` so that `Send` succeeds. The Rust
  client has no such flag: sending without a connection returns
  `SteamError::Connection`, but the job manager, the callback queue and the
  handler registry all work without a connection, so the helper only has to
  build the client.
* `connected_steam_client_with_identifier(identifier)` - the same, for a client
  created with a custom client id. It is currently **not used by any test**; it
  was written for the two `debug_log` client-id facts, which turned out not to
  be reproducible for an unrelated reason (see the `debug_log.rs` entry).
* `sha1(data) -> [u8; 20]` and `sha256(data) -> [u8; 32]` - FIPS 180-4 digests
  implemented here, because the C# uses `System.Security.Cryptography.SHA1/SHA256.HashData`
  and the test crate does not depend on a `sha1`/`sha2` crate.
* `Aes256` - a minimal AES-256 encryptor (`new`, `encrypt_block`, `encrypt_ecb`,
  `encrypt_cbc_pkcs7`) with a `const fn` S-box built from the GF(256) inverse.
  The `aes`/`cbc`/`ecb` crates are dependencies of the *library* but not of the
  test crate, so the encryptor lives here. It is used to build the input of
  `CryptoHelper::symmetric_decrypt`.

The C# `HandlerTestBase<T>` has no separate Rust file: the two facts that used
it (`steam_user.rs` and `steam_game_server.rs`) each define a local
`handler_and_client` helper.

#### 7.2.3 `tests/data/` - the binary fixtures

The eleven binary fixtures are copied verbatim from the C# tree:

| Fixture | C# origin | Used by |
|---|---|---|
| `001_in_8904_k_EMsgClientPICSProductInfoResponse_app480_metadata.bin` | `SteamKit2/Tests/Packets/` | `packet.rs` |
| `002_in_8904_k_EMsgClientPICSProductInfoResponse_app480.bin` | `SteamKit2/Tests/Packets/` | `packet.rs` |
| `003_in_8904_k_EMsgClientPICSProductInfoResponse_sub0.bin` | `SteamKit2/Tests/Packets/` | `packet.rs` |
| `depot_4_chunk_dc036a660987d8f705936d4443fedb5e1d2e5d9b.bin` | `SteamKit2/Tests/Files/` | `depot_chunk.rs`, `lzma.rs` |
| `depot_1004_chunk_b4e70afde4693284936da1a67758c897a492e1fc.bin` | `SteamKit2/Tests/Files/` | `depot_chunk.rs` |
| `depot_232250_chunk_7b8567d9b3c09295cdbf4978c32b348d8e76c750.bin` | `SteamKit2/Tests/Files/` | `depot_chunk.rs` |
| `depot_3441461_chunk_9e72678e305540630a665b93e1463bc3983eb55a.bin` | `SteamKit2/Tests/Files/` | `depot_chunk.rs` |
| `depot_440_chunk_bac8e2657470b2eb70d6ddcd6c07004be8738697.bin` | `SteamKit2/Tests/Files/` | `depot_chunk.rs` |
| `depot_440_1118032470228587934.manifest` | `SteamKit2/Tests/Files/` | `depot_manifest.rs` |
| `depot_440_1118032470228587934_v4.manifest` | `SteamKit2/Tests/Files/` | `depot_manifest.rs` |
| `depot_440_1118032470228587934_decrypted.manifest` | `SteamKit2/Tests/Files/` | `depot_manifest.rs` |

`tests/data/.gitattributes` is copied from `SteamKit2/Tests/Files/.gitattributes`
so that git does not mangle the line endings of the binary assets.

The C# `Tests/Packets/` directory holds the three `_in_` packet captures; they
are the only fixtures the C# reads from disk at runtime rather than from an
embedded resource, and `packet.rs` reads them from `tests/data` with the same
"keep every `*.bin` whose name marks it as inbound" rule.

#### 7.2.4 The ignored tests

`cargo test --workspace --no-fail-fast` reports **4 ignored**, all of them
`#[ignore]`-annotated test functions. An earlier revision of this guide counted a
fifth, the rustdoc doctest in `client_msg_handler.rs`; in this tree `cargo test`
does not count it. In detail:

1. **`cdn_client.rs` - `throws_steam_kit_web_exception_on_unsuccessful_web_response_for_manifest`**
   (`#[tokio::test] #[ignore = "requires a mock HTTP transport: ..."]`). The C#
   `CDNClientFacts.ThrowsSteamKitWebExceptionOnUnsuccessfulWebResponseForManifest`
   injects a `TeapotHttpMessageHandler` into the `HttpClient` pipeline so that
   every CDN request is answered with HTTP 418, and then asserts that
   `SteamKitWebRequestException` carries that status code. The Rust port builds
   its HTTP transport with `reqwest`, which offers no seam for swapping in a
   fake transport without adding a new dependency to this crate. The body is the
   literal port, and it fails loudly (rather than silently passing) if it is
   ever run, because the client really does try to reach `localhost:80`.
2. **`cdn_client.rs` - `throws_steam_kit_web_exception_on_unsuccessful_web_response_for_chunk`**
   (`#[tokio::test] #[ignore = "..."]`) - the same teapot handler, but for the
   chunk download path. Same reason.
3. **`depot_e2e.rs` - the live depot end-to-end test** (`#[tokio::test]
   #[ignore = "needs the network and a depot whose key the anonymous account can
   read"]`). This one is not a ported C# fact at all - it is a new live test for
   this port, described in 7.4. It needs the network *and* an account that owns
   the app, because the depot decryption key is ownership-gated.
4. **`simple_console_debug_listener.rs` - `prints_the_line_to_stdout`**
   (`#[test] #[ignore = "helper for writes_output_to_console; it is started in a
   child process"]`). This is not a ported C# fact but a *helper* for the
   ported fact `writes_output_to_console`. The C# test replaces `Console.Out`
   with a `StringWriter` and asserts the exact text the listener writes; Rust
   offers no supported way to replace the process-wide standard output, so the
   parent test re-executes the test binary with
   `--exact prints_the_line_to_stdout --ignored --nocapture` and the
   environment variable `STEAMKIT_TEST_PRINT_LINE=1`, captures the child's
   stdout and picks the `[some cat]: a message` line out of it. The helper
   itself calls `std::process::exit(0)` so that the harness does not write its
   own summary into the captured output. It is ignored so that it only ever
   runs when the parent starts it.
5. **The rustdoc doctest in `crates/steamkit/src/steam/handlers/client_msg_handler.rs`
   (line 11).** The module documentation shows the frozen shape of the
   `ClientMsgHandler` trait in a fenced code block marked `ignore`:

   ```
   //! ```ignore
   //! #[async_trait]
   //! pub trait ClientMsgHandler: Send + Sync {
   //!     async fn handle_msg(&self, client: &SteamClient, packet: &dyn IPacketMsg) -> SteamResult<()>;
   //! }
   //! ```
   ```

   rustdoc compiles code blocks marked `ignore` but never runs them, and counts
   them as *ignored* in the doctest summary. This is the fifth "ignored" in
   the cargo output, and it is **not** a test of the port at all - it is a
   documentation block that must not be compiled as a doctest, because it is
   the contract's sketch of the trait, not working code.

Note that the repository's own wording is slightly loose on this point:
§14.1 of this guide calls them all "4 `#[ignore]`d (documented inline)", and
the porting notes §9.1 correctly names only the *two C# tests* that are ported as
`#[ignore]`d - `CDNClientFacts` (two facts) and the
`SimpleConsoleDebugListenerFacts` helper. The remaining two entries are the
`depot_e2e.rs` live test (Rust-only) and the doctest.

#### 7.2.5 Current result

```
$ cargo test --workspace --no-fail-fast
...
test result: ok. 332 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out
```

**332 passed, 0 failed, 5 ignored** when this section was written. The current
suite is **458 passed, 0 failed, 4 ignored across 73 binaries** (measured
2026-11-04); the breakdown below explains the composition, not the current total.
At the time, the arithmetic reconciled exactly:

* 328 test functions in `crates/steamkit-tests` (262 `#[test]` + 66 `#[tokio::test]`),
  of which 4 are `#[ignore]`d -> 324 pass;
* plus 8 unit tests inside the library crates (`steamkit`: 3 in
  `base/generated/steam_language_internal.rs`; `steamkit-nethook`: 1 in
  `logger.rs`; `steamkit-tools`: 4 in `nethook/item.rs`) -> **332 passed**;
* plus the 4 `#[ignore]`d test functions, and the 1 rustdoc doctest that the
  earlier tooling counted -> **5 ignored** at the time (see 7.2.4; the current
  run reports 4).

The C# xunit facts map one-to-one onto the Rust `#[test]` functions, with one
mechanical transformation: every C# `[Fact]` becomes exactly one Rust test
function, and every C# `[Theory]` is expanded into one Rust test function per
data row, with a name that spells out the row
(`returns_correct_when_empty_0/1/2`, `matches_reference_1024_plus_15`,
`is_private_address_ipv4_10_0_0_1`, `to_chat_id_only_supports_clans_individual`,
...). The C# tree carries 255 `[Fact]` methods and 13 `[Theory]` methods; all 13
theories live in three files and carry 72 `[InlineData]` rows in total
(`SteamIDFacts.cs` 39, `Adler32Facts.cs` 24, `CDNClientFacts.cs` 9) plus one
`[MemberData]` theory (`Adler32Facts.VectorizedMatchesScalar`, driven by
`VectorizedLengths`), i.e. 73 theory-derived Rust tests. The Rust total
reconciles exactly:

```
255  C# [Fact] methods
-  3  not ported (see the three exceptions below)
+  2  Rust-only enum-drift guards in steam_id.rs
+  1  Rust-only live test in depot_e2e.rs
+ 73  theory-derived tests (72 inline-data rows + 1 member-data theory)
= 328  Rust test functions
```

`steam_id.rs` is the extreme case: 23 C# facts + 6 theories with 39 rows become
62 Rust tests, and the arithmetic (23 + 39 = 62) matches exactly.

Three deliberate exceptions to the one-to-one rule, each documented at the top
of the file it lives in:

* `server_record.rs` - the C# `ServerRecordFacts.NullsAreEqualOperator` is **not
  ported**: Rust has neither null nor reference equality for `ServerRecord`,
  which is a value type, so there is nothing to assert. 8 C# facts -> 7 Rust
  tests.
* `steam_configuration.rs` - the C# `SteamConfigurationDefaultFacts.AllowsDirectoryFetch`
  has no dedicated Rust test; the assertion survives only in the *configured*
  variant `directory_fetch_is_configured`, which asserts
  `!allow_directory_fetch()`. 24 C# facts -> 23 Rust tests.
* `key_value.rs` - the C# `CanReadAndIgnoreConditionals` is inside `#if false`
  in the C# tree, so it is disabled there and is not ported either. It is the
  **one** C# fact in the whole tree that does not run: 255 fact methods, of
  which 254 are active.

Two further deviations are *additions* rather than omissions: `steam_id.rs` has
two extra tests (`e_universe_enum_has_not_changed`, `e_account_type_enum_has_not_changed`)
that guard the generated enums against drift, and `adler32.rs` compares the
library against a reference implementation rather than against the C#'s SIMD
paths, because the Rust port implements the scalar path only.

#### 7.2.6 The test files

##### `crates/steamkit-tests/tests/adler32.rs` - 275 lines
**C# counterpart:** `SteamKit2/Tests/Adler32Facts.cs`
**Purpose:** The Adler-32 checksum, against reference values, known vectors and
overflow boundaries.
**How it works:** the C# file is wrapped in `#if DEBUG`, so its facts only run
in a debug build there; Rust test binaries have no such split, so every fact
runs unconditionally. The C# class has three implementations of the same
algorithm (a scalar one and two SIMD ones, `Vector128`/`Vector256`); the Rust
port implements the scalar path only, so the facts that compared the vectorized
paths against the scalar one compare the library against a straightforward
reference implementation written in the test file instead. The asserted values
are unchanged. 27 tests: `returns_correct_when_empty_{0,1,2}`,
`matches_reference_*` (15 rows), `matches_known_vectors_{a,wikipedia,digits,sentence}`,
`no_overflow_with_max_bytes`, `vectorized_matches_scalar`,
`vectorized_matches_scalar_on_random_slices`, and
`vectorized_no_overflow_with_max_bytes_{fff0fff0,ffffffff}`.
**Notes and gotchas:** the "vectorized" tests are the C#'s SIMD facts renamed;
they no longer test SIMD but they still test the same boundaries, and they are
the only place where the port's own reference implementation is the oracle.

##### `crates/steamkit-tests/tests/async_job.rs` - 666 lines
**C# counterpart:** `SteamKit2/Tests/AsyncJobFacts.cs`
**Purpose:** The async job machinery - job registration, completion, timeout,
cancellation and the "multiple" variant.
**How it works:** 22 `#[tokio::test]`s, split between the single-job facts
(`async_job_*`) and the multiple-job facts (`async_job_multiple_*`). Two local
callback types (`with_job_id`/`with_is_finished`) implement `CallbackMsg`. The
C# file wraps most of its facts in `#if DEBUG`; Rust has no such split. Three
C# constructs need a Rust counterpart of their own: the C# `AsyncJob<T>`
constructor registers the job with the client's job manager, but the Rust
`SteamClient::start_job` *additionally* fails the job right away when the
client is not connected (which is what the first fact asserts), so the
remaining facts create their jobs through `AsyncJobManager::start_job` and
`AsyncJobManager::start_job_multiple`, which only register them; the C#
`ConnectedSteamClient` helper only marks the client as connected, and the Rust
client has no such flag and the job manager does not care about the connection
state; and the C# facts set a 50 ms per-job timeout, but the Rust job manager
does not expose a per-job timeout and uses `MANAGER_JOB_TIMEOUT` for every job,
so the timeout facts have to wait for that (much longer) deadline.
**Notes and gotchas:** the timeout facts are the slow ones in the suite. The
`async_job_throws_exception_on_null_callback` /
`..._throws_failure_exception_on_failure` facts assert the Rust error variants
that stand in for the C# `AsyncJobFailedException` and `TaskCanceledException`.
`async_job_multiple_extends_timeout_on_message` used to assert something it
could not possibly distinguish — see 7.2.7.

##### `crates/steamkit-tests/tests/callback_manager.rs` - 706 lines
**C# counterpart:** `SteamKit2/Tests/CallbackManagerFacts.cs`
**Purpose:** The callback manager - subscription, dispatch by job id,
disposal, synchronous and asynchronous subscribers.
**How it works:** 19 tests (9 `#[test]`, 10 `#[tokio::test]`), with a local
callback type that carries an optional job id. The C# `CallbackManager` is
constructed with a `SteamClient` and pulls the callbacks it dispatches out of
the client's callback queue; the Rust `CallbackManager` is standalone - the
client hands every posted callback to `CallbackManager::dispatch`, so
subscribing is enough to receive them. Three C# constructs have no direct Rust
counterpart: `Subscribe<CallbackMsg>` in the C# registers a handler for *every*
callback, because the C# manager dispatches with
`CallbackType.IsAssignableFrom(type)`, whereas the Rust manager compares the
exact `TypeId` of the callback, so a "catch all" subscription cannot be
expressed; `RunCallbacks` in the C# blocks until an asynchronous subscriber has
finished, whereas the Rust `dispatch` spawns asynchronous subscribers onto the
runtime, so the facts that depend on the blocking behaviour use
`CallbackManager::dispatch_async`; and an asynchronous subscriber that throws
cannot report its error, because the Rust handler returns `()`.
**Notes and gotchas:** the last two facts make that last point concrete -
`async_callback_exception_propagates_on_async_path` (the error surfaces
through the async path) and
`async_callback_exception_wrapped_in_aggregate_on_sync_path` (the sync path
wraps it). `posted_callback_triggers_action_catch_all` is the fact that shows
the missing catch-all subscription.

##### `crates/steamkit-tests/tests/cdn_client.rs` - 196 lines
**C# counterpart:** `SteamKit2/Tests/CDNClientFacts.cs`
**Purpose:** The CDN client - argument validation, error mapping and the
private-address check.
**How it works:** 14 tests (9 `#[test]`, 5 `#[tokio::test]`). `localhost_server()`
builds the C# `Server { Protocol = HTTP, Host = "localhost", Port = 80 }` through
`Server::from_dns_end_point("localhost", 80)`, which speaks HTTP unless the
port is 443. `parse_ip` reproduces `IPAddress.Parse`, and
`assert_teapot_response` asserts the C#'s
`Assert.Equal((HttpStatusCode)418, ex.StatusCode)` by looking for "418" inside
the `SteamError::Other` message, because the Rust port renders the status code
into the message rather than carrying a typed status. The nine
`is_private_address_*` tests are the expansion of the two C# theories
`IsPrivateAddress_IPv4Tests` (5 rows) and `IsPrivateAddress_IPv6Tests` (4 rows).
**Notes and gotchas:** two of the tests are `#[ignore]`d - see 7.2.4. The
remaining three `#[tokio::test]`s (no chunk id, destination buffer too small
with and without a depot key) need **no network at all**, because the Rust port
validates its arguments before it builds the request; the C# relied on the
`ArgumentException.ParamName` instead.

##### `crates/steamkit-tests/tests/client_msg.rs` - 98 lines
**C# counterpart:** `SteamKit2/Tests/ClientMsgFacts.cs`
**Purpose:** Building a `ClientMsgProtobuf` and reading a null-terminated
string out of its payload.
**How it works:** 2 tests, `build_struct_msg` and
`payload_reader_reads_null_term_string` /
`payload_reader_does_not_overflow_past_null_term_string`. The C# file is
wrapped in `#if DEBUG`; Rust has no such split, so every fact runs
unconditionally.
**Notes and gotchas:** the third test is the one that matters - the payload
reader must not read past the null terminator into the next field.

##### `crates/steamkit-tests/tests/cm_client.rs` - 213 lines
**C# counterpart:** `SteamKit2/Tests/CMClientFacts.cs`
**Purpose:** `CMClient.GetPacketMsg` - turning a raw buffer into the right
concrete packet type.
**How it works:** 5 tests. The C# file is wrapped in `#if DEBUG` and marked as
part of the `NotThreadSafeResourceCollection`, because `CMClient.GetPacketMsg`
writes to the shared debug log; the Rust port needs no such collection, since
every test builds its own `SteamClient`. The C# asserts with
`Assert.IsAssignableFrom<T>`; the Rust `IPacketMsg` has no `as_any` and the
concrete packet types keep their fields private, so a `Box<dyn IPacketMsg>`
cannot be downcast to `PacketMsg` / `PacketClientMsg` / `PacketClientMsgProtobuf`.
The type assertions are therefore ported as the strongest set of observable
properties plus - where the payload allows it - a structural proof: the payload
is re-parsed through the candidate concrete types, and the candidates that
could not possibly have produced the packet are ruled out.
**Notes and gotchas:** the helpers `or_packet` and `serialize_header` build the
two message shapes the C# uses; `assert_is_packet_msg`,
`assert_is_packet_client_msg` and `assert_is_packet_client_msg_protobuf` are the
substitute for the C#'s `IsAssignableFrom` assertions.

##### `crates/steamkit-tests/tests/crypto_helper.rs` - 47 lines
**C# counterpart:** `SteamKit2/Tests/CryptoHelperFacts.cs`
**Purpose:** `CryptoHelper::symmetric_decrypt` - the AES-256-CBC format SteamKit
uses, with an ECB-encrypted IV prefix.
**How it works:** one test, `test_symmetric_encryption`. It builds a random
AES-256-CBC ciphertext with the ECB-encrypted IV prefix using the
`common::Aes256` helper, and asserts that `symmetric_decrypt` recovers the
plaintext.
**Notes and gotchas:** this is the fact that caught the worst bug in the port -
`symmetric_decrypt` allocated a zeroed buffer and never copied the ciphertext
in, so it had never decrypted anything (see 7.3).

##### `crates/steamkit-tests/tests/date_utils.rs` - 65 lines
**C# counterpart:** `SteamKit2/Tests/DateUtilsFacts.cs`
**Purpose:** The round trip of a unix timestamp and of the exact round-trip
date format (`DateTime.ToString("o")`).
**How it works:** one test, `roundtrips_datetime_utc`. The Rust port returns a
`SystemTime`, which carries neither a "kind" nor a formatter, so the test
formats the timestamp itself; `chrono` is not a dependency of the test crate,
hence the small civil-calendar conversion (`civil_from_days`,
`format_round_trip_utc`) in the file.
**Notes and gotchas:** the helper is a hand-rolled days-to-civil-date
conversion. It is the only place in the suite where a calendar algorithm is
re-implemented rather than imported.

##### `crates/steamkit-tests/tests/debug_log.rs` - 248 lines
**C# counterpart:** `SteamKit2/Tests/DebugLogFacts.cs`
**Purpose:** The process-wide `DebugLog` - listeners, the enabled flag,
parameter formatting and the client-id prefix.
**How it works:** 9 tests, with a local listener type implementing
`IDebugListener` and a `DebugLogTestGuard` that mirrors the C#
`DebugLogSetupTeardownAttribute` (`ClearListeners` before the test, then
`Enabled = false` and `ClearListeners` after). `DebugLog` is a process-wide
global that is not safe to exercise from several tests at once; the C# suite
puts these tests in a `NotThreadSafeResourceCollection` so xunit runs them
sequentially. Cargo runs the tests of one binary in parallel threads, so a
process-wide mutex (`DEBUG_LOG_LOCK`) serialises them here. (Other test files
live in separate binaries, so cross-file interference is impossible.) The
listener records the number of calls and the last `(category, msg)` pair it
saw; the facts cover the action listener, the debug listener, the disabled
state, adding and removing a listener, clearing listeners, the two-argument
`write_line` overload (which writes the message verbatim, so `"{0}"` survives)
and the composite-formatting overload.
**Notes and gotchas:** the last two tests,
`generated_cm_client_id_prefixed` and `custom_cm_client_id_prefixed`, used to be
**empty on purpose**. The C# asserts that `new SteamClient().LogDebug("MyCategory",
"My {0}st message", 1)` reaches a `DebugLog` listener with the category
prefixed by the client id. The Rust `SteamClient::log_debug` forwards to
`CMClient::log_debug`, which used to emit a `tracing::debug!` event carrying
`client` and `category` as separate structured fields: it did **not** call
`DebugLog::write_line`, did **not** prefix the category with the client id and
had no args parameter. A `DebugLog` listener was therefore never notified and
the C# assertion could not be reproduced - the two tests documented that
discrepancy as a library bug report rather than asserting anything, and they
were the only tests in the suite that passed without asserting. **Both now
assert**, because `CMClient::log_debug` was changed to do what the C# does:
build the full category `"{id}/{category}"`, write the line to the global
`DebugLog` sink, and emit the `tracing` event next to it. See 7.2.7. The file
also notes that the global state is reset on every acquire anyway, so the
poisoning of the mutex is deliberately ignored.

##### `crates/steamkit-tests/tests/depot_chunk.rs` - 546 lines
**C# counterpart:** `SteamKit2/Tests/DepotChunkFacts.cs`
**Purpose:** Depot chunk decryption and the three container formats - PKZip,
VZip and VZstd.
**How it works:** 15 tests over the five chunk fixtures in `tests/data`. The
helpers `depot4_chunk` and `depot1004_chunk` build the two depot key versions;
`read_chunk_file`, `hex_upper`, `decrypt_depot_chunk`,
`read_decrypted_v_zip_chunk`, `with_stream` and `with_decompressed_size` are
the shared fixtures. The tests decrypt and decompress a chunk in each container
format and assert the round trip
(`decrypts_and_decompresses_depot_chunk_pkzip`, `..._vzip`, `..._zstd`, plus
the "larger" variants for the bigger fixtures).
The VZip error paths are covered individually
(`throws_on_corrupt_depot_chunk_vzip`, `vzip_throws_on_truncated_stream`,
`vzip_throws_when_output_is_larger_than_stream`,
`vzip_throws_on_invalid_header_footer_or_properties`,
`vzip_throws_on_match_before_any_data`, `vzip_ignores_dictionary_size_smaller_than_output`,
`vzip_cuts_matches_at_end_of_output`) and the VZstd ones likewise
(`zstd_throws_on_corrupt_frame`, `zstd_throws_on_short_input`). The C# file is
wrapped in `#if DEBUG` for the second half (the VZip and VZstd container facts);
Rust has no such split, so every fact runs unconditionally. The C#
`DepotChunk.Process` returns the number of written bytes and throws
`InvalidDataException` on bad input; the Rust port returns
`Result<usize, SteamError>`, so `Assert.Throws<InvalidDataException>` becomes a
match on the error variant the port reports.
**Notes and gotchas:** this is the fact family that caught the second
zeroed-buffer bug (the IV was decrypted from a zeroed `iv` instead of
`data[..16]`) and the `lzma` `bit - 1` underflow.

##### `crates/steamkit-tests/tests/depot_manifest.rs` - 216 lines
**C# counterpart:** `SteamKit2/Tests/DepotManifestFacts.cs`
**Purpose:** Parsing and decrypting depot manifests, and the binary
serialisation round trip.
**How it works:** 5 tests over the three manifest fixtures in `tests/data`:
`parses_and_decrypts_manifest_version4`, `parses_and_decrypts_manifest`,
`parses_decrypted_manifest`,
`roundtrip_serializes_manifest_encrypted_manifest` and
`roundtrip_serializes_manifest_byte_identical`. The C# suite reads the
manifests through `Assembly.GetManifestResourceStream("Tests.Files.*")`; the
Rust suite reads the very same files from `tests/data` (see
`tests/common/mod.rs`).
**Notes and gotchas:** the C# `Path.Join("bin", "dxsupport.cfg")` builds a path
with the separator of the current platform, and the depot manifest replaces the
separator Valve uses with the platform one, so the expected names are built
with `MAIN_SEPARATOR_STR` as well. The byte-identical round trip is the fact
that caught the key/IV swap in `new_from_slices`.

##### `crates/steamkit-tests/tests/file_storage_server_list_provider.rs` - 71 lines
**C# counterpart:** `SteamKit2/Tests/FileStorageServerListProviderFacts.cs`
**Purpose:** The file-backed server list provider.
**How it works:** one `#[tokio::test]`, `reads_updated_server_list`, with a
`temporary_filename` helper.
**Notes and gotchas:** **deviation from the C# suite** - the C# test stores its
list in `servertest.bin` in the working directory; the Rust port writes into a
uniquely named file inside the system temporary directory (the file name embeds
a unique suffix) so that the test does not pollute the repository, and removes
the file once the test is done.

##### `crates/steamkit-tests/tests/game_id.rs` - 52 lines
**C# counterpart:** `SteamKit2/Tests/GameIDFacts.cs`
**Purpose:** The `GameID` CRC and hash-code behaviour.
**How it works:** 3 tests - `mod_crc_correct`, `shortcut_crc_correct` and
`game_ids_k_hashcode_equality`.
**Notes and gotchas:** the smallest file in the suite; a direct port with no
deviations.

##### `crates/steamkit-tests/tests/http_client_purpose.rs` - 91 lines
**C# counterpart:** `SteamKit2/Tests/HttpClientPurposeConfiguredFacts.cs`
**Purpose:** The `HttpClientPurpose` that the HTTP client factory is invoked
with - `WebAPI`, `CMWebSocket` and `CDN`.
**How it works:** 4 tests, with a `purpose_recording_configuration` helper
that records the purpose the factory is invoked with. The C# factory reflects
the `HttpClientPurpose` it is invoked with into an `X-Purpose` default request
header, and the test reads that header back off the returned `reqwest::Client`.
The test crate cannot construct a `reqwest::Client` with custom default headers
(`reqwest` is not a direct dependency and is not re-exported by `steamkit`), so
the port records the purpose the factory is invoked with and asserts on that
recorded value - the same value the C# `X-Purpose` header would carry.
**Notes and gotchas:** this is the test that pins the `HttpClientPurpose`
variants, including the `CMWebSocket` variant that the default connection
factory currently does not honour (see the porting notes §10).

##### `crates/steamkit-tests/tests/isolated_storage_server_list_provider.rs` - 56 lines
**C# counterpart:** `SteamKit2/Tests/IsolatedStorageServerListProviderFacts.cs`
**Purpose:** The isolated-storage-backed server list provider.
**How it works:** one `#[tokio::test]`, `reads_updated_server_list`.
**Notes and gotchas:** **deviation from the C# suite** - the C# provider stores
the list in a .NET `IsolatedStorageFile`
(`IsolatedStorageFile.GetUserStoreForAssembly()`), a Windows-only concept with
no Rust equivalent. The Rust port persists the very same `protobuf-net` byte
layout to `<temp dir>/steamkit/serverlist.protobuf`, so the assertions of the
C# test carry over unchanged. The file is removed once the test is done.

##### `crates/steamkit-tests/tests/key_value.rs` - 730 lines
**C# counterpart:** `SteamKit2/Tests/KeyValueFacts.cs`
**Purpose:** The `KeyValue` type - construction, indexing, text and binary
serialisation, binary deserialisation, and the text reader's encoding
handling.
**How it works:** 33 tests (30 `#[test]`, 3 `#[tokio::test]`) - the largest
suite in the port after `steam_id.rs`. The C# tests write a couple of temporary
files; the Rust port writes the same files into the system temporary directory
and removes them again (`temporary_file_name`). Four C# constructs have no
direct Rust counterpart and are spelled out in the file header:
`KeyValueIndexerUpdatesKey` asserts that the *caller's* `KeyValue` object was
renamed by the indexer, which relies on `KeyValue` being a reference type in
C# - the Rust `KeyValue` is a value type, so only the stored copy is renamed;
`KeyValues_TryReadAsBinary_ReadsBinary` and
`KeyValuesReadsBinaryWithLeftoverData` assert the position of the input
stream after the read, and the Rust binary reader buffers its input, so those
tests feed it through a reader that hands out a single byte per call, which
makes the number of consumed bytes observable again; `SetFromString(null, ...)`
style nullable string arguments do not exist, and `KeyValue::load_from_string`
returns an `Option` instead of throwing; and the C# `#if false` test
`CanReadAndIgnoreConditionals` is disabled in C# and therefore not ported.
**Notes and gotchas:** the UTF-16 handling is the interesting part:
`utf16_le_with_preamble` and `utf16_be_with_preamble` build the two UTF-16
documents that the C# `StreamReader` detects by their byte order mark, and
`can_load_text_documents_with_a_byte_order_mark` covers the UTF-8 mark. The
C# `StreamReader` sniffs UTF-8, UTF-16-LE and UTF-16-BE; the Rust port
originally only stripped the UTF-8 mark, and that bug was caught here.

##### `crates/steamkit-tests/tests/lzma.rs` - 263 lines
**C# counterpart:** `SteamKit2/Tests/LzmaFacts.cs`
**Purpose:** The LZMA decoder and the VZip delta filter.
**How it works:** 4 tests - `v_zip_delta_decodes_against_source_chunk`,
`v_zip_delta_decodes_with_source_chunk_larger_than_dictionary`,
`v_zip_delta_throws_with_wrong_source_chunk` and
`lzma_util_decompresses_blob`. The helpers build the fixtures on the fly:
`crc32`, `read_depot_chunk_contents`, `create_encoder`, `create_delta_source`,
`create_delta_target` and `create_v_zip_delta`. The C# file is wrapped in
`#if DEBUG`; Rust has no such split. Two C# constructs need a Rust counterpart
of their own:
`Crc32.HashToUInt32` comes from `System.IO.Hashing`, which the test crate does
not depend on, so the CRC-32 is computed in the file; and
`new Random(1950).NextBytes(..)` fills the middle of the delta target with
incompressible bytes - the concrete pseudo-random generator does not matter for
the facts below (they only assert that the delta is small and that it round
trips), so the `rand` crate is used instead.
**Notes and gotchas:** the delta tests reuse the
`depot_4_chunk_dc036a660987d8f705936d4443fedb5e1d2e5d9b.bin` fixture from
`tests/data`.

##### `crates/steamkit-tests/tests/machine_info.rs` - 298 lines
**C# counterpart:** `SteamKit2/Tests/MachineInfoFacts.cs`
**Purpose:** The machine-info provider and the cached machine id.
**How it works:** 8 tests (7 `#[test]`, 1 `#[tokio::test]`) with several local
provider types: one that counts invocations, one that returns fixed values, and
one that panics. The C# file is compiled `#if DEBUG`, so the whole file only
exists in debug builds; every test in it is ported here unconditionally. Three
C# constructs have no direct Rust counterpart: `ProviderIsNotRetained` relies
on the garbage collector collecting the provider after `Setup()` returns - Rust
has no GC, so the test drops the last `Arc` and checks the weak reference the
generation table holds; `ExceptionBubblesUp` expects
`HardwareUtils.GetMachineID` to rethrow the exception the provider raised - the
Rust `IMachineInfoProvider` trait returns `Option<Vec<u8>>` instead of throwing,
so a provider that only fails panics, and the test asserts that the panic
reaches the caller with the very same message; and `GenerationIsThreadSafe`
uses a `ManualResetEventSlim` to release all threads at once, which a
`std::sync::Barrier` does here.
**Notes and gotchas:** the `exception_bubbles_up` fact is written with
`std::panic::catch_unwind`; it is the only test in the suite that asserts on a
panic.

##### `crates/steamkit-tests/tests/net_helpers.rs` - 148 lines
**C# counterpart:** `SteamKit2/Tests/NetHelpersFacts.cs`
**Purpose:** The IP-address helpers - reading the address out of a message,
obfuscating a private address, parsing an endpoint.
**How it works:** 6 tests with two local helpers, `v4` and `v6`, that build
`IpAddr` values. The C# file is wrapped in `#if DEBUG`; Rust has no such
split.
**Notes and gotchas:** `obfuscate_private_ip` is the helper that Steam uses to
hide the local address in the logon message.

##### `crates/steamkit-tests/tests/network_listener.rs` - 79 lines
**C# counterpart:** `SteamKit2/Tests/NetworkListenerFacts.cs`
**Purpose:** The debug network listener - that an outgoing message is reported
to it.
**How it works:** one test, `network_listener_calls_on_outgoing_message`, with
a local listener implementing the network-listener trait. The C# test fakes
`CMClient.IsConnected = true` via reflection so that `Send()` succeeds. The
Rust `CMClient::send` notifies the debug listener *before* it fails with "not
connected", so the send error is ignored and the listener is inspected instead.
**Notes and gotchas:** the file header marks this as a documented deviation; the
C# needed reflection to reach a private field, the Rust port needs nothing of
the sort.

##### `crates/steamkit-tests/tests/packet.rs` - 126 lines
**C# counterpart:** `SteamKit2/Tests/PacketFacts.cs`
**Purpose:** The end-to-end handler pipeline: a captured packet must produce
exactly the callback the message type predicts.
**How it works:** one `#[tokio::test]`, `posts_expected_callbacks`. The C# test
walks the `Tests/Packets` folder, keeps every `*.bin` file whose name marks it
as inbound (`..._in_...`), builds a packet message from it and feeds it to a
`SteamClient`, asserting that the handler pipeline posts exactly the callback
the C# `GetCallback` switch predicts for the message type encoded in the file
name. The Rust port reads the same fixtures from `tests/data`: `get_packets`
sorts the directory for a deterministic order, `get_packet` splits the file stem
on `_` and reads the direction and the `EMsg` value out of it, and
`get_callback` is the C# `GetCallback(EMsg)` switch. The Rust `IPacketMsg` does
expose `as_any`, so the callback type assertion *is* a real downcast
(`ExpectedCallback::matches`).
**Notes and gotchas:** the fact also asserts that the packet is a protobuf
packet and that the payload re-parses identically through
`PacketClientMsgProtobuf::new` - the structural counterpart of the C#'s
`Assert.IsType<PacketClientMsgProtobuf>(packetMsg, exactMatch: false)`.

##### `crates/steamkit-tests/tests/server_record.rs` - 198 lines
**C# counterpart:** `SteamKit2/Tests/ServerRecordFacts.cs`
**Purpose:** The `ServerRecord` value type - equality, hashing, and the socket
server factories.
**How it works:** 7 tests with the helpers `host_address`, `create_server`,
`create_websocket_server` and `hash_of`. The C# `ServerRecord.CreateServer(
"host", 1, ...)` and `ServerRecord.CreateWebSocketServer("host:1")` factories
build a `DnsEndPoint` without resolving anything, while the Rust factories are
`async` and perform a real DNS lookup, so the records are built directly with
`ServerRecord::new(EndPoint::dns(..))`, which produces the exact same value
without touching the network. `Assert.Equal(l.GetHashCode(),
r.GetHashCode())` is ported with `std::collections::hash_map::DefaultHasher`, the
hasher Rust's own collections use for `ServerRecord` keys.
**Notes and gotchas:** the C# `NullsAreEqualOperator` is **not ported** (see
7.2.5). The `!= null` / `== null` / `Equals(null)` assertions of
`null_is_not_equal` are dropped; what remains is the assertion that the record
is a usable value with the expected endpoint and protocol types.

##### `crates/steamkit-tests/tests/simple_console_debug_listener.rs` - 68 lines
**C# counterpart:** `SteamKit2/Tests/SimpleConsoleDebugListenerFacts.cs`
**Purpose:** The `SimpleConsoleDebugListener` writes `[category]: message` to the
console.
**How it works:** 2 tests. `prints_the_line_to_stdout` is an `#[ignore]`d
*helper* that runs in a child process; `writes_output_to_console` is the
ported C# fact and drives the helper. The C# test replaces `Console.Out` with a
`StringWriter` and asserts the exact text the listener writes. Rust offers no
supported way to replace the process-wide standard output, so the listener runs
in a child process (the test binary re-executed with a filter) whose standard
output the parent captures. The parent asserts the child's exit status, picks
the `[some cat]` line out of the captured stdout and compares it with
`"[some cat]: a message"`.
**Notes and gotchas:** the child is started with
`--exact prints_the_line_to_stdout --ignored --nocapture` and the environment
variable `STEAMKIT_TEST_PRINT_LINE=1`; the helper calls `std::process::exit(0)`
before the harness writes its own summary, so the captured output holds nothing
but the listener's line. This is one of the four "ignored" entries - see
7.2.4.

##### `crates/steamkit-tests/tests/smart_cm_server_list.rs` - 480 lines
**C# counterpart:** `SteamKit2/Tests/SmartCMServerListFacts.cs`
**Purpose:** The smart CM server list - candidate selection, ordering, marking
and merging.
**How it works:** 11 tests (1 `#[test]`, 10 `#[tokio::test]`) with the helpers
`configuration`, `server_list`, `socket_server`, `websocket_server`,
`replace_list`, `try_mark` and `next_candidate`. The C# suite is wrapped in
`[Collection(nameof(NotThreadSafeResourceCollection))]` because the class under
test mutates process-wide state; in Rust every test builds its own
`SmartCMServerList` from its own `SteamConfiguration`, so the collection is
unnecessary and the tests are fully independent. Three deviations are
documented in the file header: the C# `ServerRecord.CreateWebSocketServer(
"host:port")` factory stores the host name and the port in a `DnsEndPoint` and
only resolves the host name when a connection is made, while the Rust factory
resolves it up front, because `SUMMARY.md` §8 requires connections to be
made to a `SocketAddr` - the tests therefore call the library factory and await
it, using host names that resolve locally (`localhost` from the hosts file, the
rest IP literals); the C# `GetNextServerCandidate` blocks on a background fetch
of the server list, while the Rust port cannot block, so the synchronous
`SmartCMServerList::get_next_server_candidate` returns the best candidate of
the list that is currently known - every test in the C# file populates the list
first and then only relies on the in-memory ordering, so the two are
observably identical; and the C# `Thread.Sleep(10ms)` between the dequeue and
the mark of a candidate is ported to `tokio::time::sleep`.
**Notes and gotchas:** the "biased towards server ordering" and "iterate all
candidates" facts are the ones that pin the candidate-selection policy.

##### `crates/steamkit-tests/tests/steam_client.rs` - 202 lines
**C# counterpart:** `SteamKit2/Tests/SteamClientFacts.cs`
**Purpose:** The `SteamClient` message-handler registry and the job id
generator.
**How it works:** 12 tests with a local `TestMsgHandler` (the C#'s
`TestMsgHandler`). The facts cover: the initial handler set, `add_handler` /
`remove_handler` (by type and by instance), the duplicate-handler error, the
no-op removal of an unregistered type, `get_required_handler` with and without a
registered handler, and the four job-id properties - thread safety, process id
zeroed, process start time filled in, box id zeroed.
**Notes and gotchas:** the job-id facts are the ones that pin the wire format of
the job id (`process id | process start time | box id | sequence`), which the
C# builds in `SteamClient.GetNextJobID`.

##### `crates/steamkit-tests/tests/steam_configuration.rs` - 367 lines
**C# counterpart:** `SteamKit2/Tests/SteamConfigurationFacts.cs`
**Purpose:** The default and the configured `SteamConfiguration`.
**How it works:** the C# file holds two test classes; each becomes a module
here: `mod steam_configuration_default` mirrors
`SteamConfigurationDefaultFacts` and `mod steam_configuration_configured_object`
mirrors `SteamConfigurationConfiguredObjectFacts`. 23 tests in total (22
`#[test]`, 1 `#[tokio::test]`), each module with its own `configuration()`
helper that builds the configuration under test. The default module covers the
cell id being zero, the five-second connection timeout, the default persona
state flags, the default HTTP client factory, the default machine info
provider, the "nothing fancy" server list provider, the server list, the
default protocols, the public universe, the Web API address and the absent Web
API key. The configured module repeats the list against a configuration built
with `with_directory_fetch(false)`, `with_cell_id(123)`, a one-minute timeout,
`with_universe(EUniverse::Internal)`, a custom Web API base address and key,
and a custom protocol set.
**Notes and gotchas:** the `default_http_client_factory` and
`http_client_factory_is_configured` facts cannot read a default request header
off a `reqwest::Client`, because the test crate cannot build one with custom
default headers; they assert on the recorded purpose instead. The
`default_machine_info_provider` fact uses the same trick - the C# asserts
`Assert.IsNotType<DefaultMachineInfoProvider>`, the Rust port checks the
provider's observable behaviour. The C# `AllowsDirectoryFetch` of the *default*
configuration has no dedicated Rust test; only the configured variant does -
see 7.2.5.

##### `crates/steamkit-tests/tests/steam_game_server.rs` - 62 lines
**C# counterpart:** `SteamKit2/Tests/SteamGameServerFacts.cs`
**Purpose:** The `SteamGameServer` handler.
**How it works:** 2 tests with a local `handler_and_client` helper (the
equivalent of the C# `HandlerTestBase<SteamGameServer>`, which builds a
`SteamClient` and fetches its handler): `log_on_posts_logged_on_callback_when_no_connection`
and `log_on_anonymous_posts_logged_on_callback_when_no_connection`.
**Notes and gotchas:** the callbacks asserted are the shared
`SteamUser.LoggedOnCallback` - the game server handler posts the same type.
Both facts assert that the handler posts the callback *even though the client
is not connected*, which is the C# behaviour.

##### `crates/steamkit-tests/tests/steam_id.rs` - 771 lines
**C# counterpart:** `SteamKit2/Tests/SteamIDFacts.cs`
**Purpose:** The `SteamID` type - construction, parsing, rendering, equality,
hashing, chat-id conversion, clan-id conversion, and the account-type and
universe enumerations.
**How it works:** 62 tests, the largest file in the suite. The C# has 23
`[Fact]`s and 6 `[Theory]`s with 39 inline-data rows; the theories are
expanded into one Rust test per row, which is where the extra 39 tests come
from (23 + 39 = 62). The local helpers are `has_chat_instance_flag` and
`steam_id_with_raw`. Two C# constructs have no direct Rust equivalent and are
spelled out in the file header: the C# tests cast out-of-range integers to
`EAccountType` / `EUniverse` (for example `(EAccountType)(-1)`), and the
generated Rust enums only hold the eleven / five documented variants, so those
ids are assembled from their raw 64-bit form, which is exactly the bit pattern
the C# code leaves behind (`SteamID::render` renders them like C# does); and
`Assert.Same` style reference equality and `Equals(object)` have no
counterpart in Rust, so the value-based assertions are ported.
**Notes and gotchas:** two of the tests are guards against enum drift -
`e_universe_enum_has_not_changed` and `e_account_type_enum_has_not_changed`.
The `steam2_render_is_valid` / `renders_steam3_by_default` pair is where the
`render(false)` bug of 7.3 shows up: the C#'s `RenderSteam2` ends in
`Convert.ToString(this)`, which binds to the **`ulong`** overload, so it
renders the numeric 64-bit id, not the Steam3 form.

##### `crates/steamkit-tests/tests/steam_user.rs` - 152 lines
**C# counterpart:** `SteamKit2/Tests/SteamUserFacts.cs`
**Purpose:** The `SteamUser` handler - the argument validation of `log_on` and
the `LoggedOnCallback` it posts.
**How it works:** 8 tests with a local `handler_and_client` helper (the
equivalent of the C# `HandlerTestBase<SteamUser>`). Five of them assert the
argument validation: no details, only a password, only an access token, and
neither a password nor an access token all fail; a username with a password
and a username with an access token both succeed. The other three assert that
`log_on` and `log_on_anonymous` post a `LoggedOnCallback` even when the client
is not connected.
**Notes and gotchas:** the C# `Assert.Throws<ArgumentException>` becomes a
`Result`/`Err` match in Rust; the five validation facts are the ones that pin
the `LogOnDetails` contract that every other sample depends on.
`log_on_throws_exception_if_details_not_provided` used to have an empty body
for the same reason the C# fact cannot be ported literally — Rust has no null
value — and **now asserts** the behaviour that does exist (see 7.2.7).

##### `crates/steamkit-tests/tests/stream_helpers.rs` - 159 lines
**C# counterpart:** `SteamKit2/Tests/StreamHelpersFacts.cs`
**Purpose:** The null-terminated string helpers on streams.
**How it works:** 2 tests (1 `#[test]`, 1 `#[tokio::test]`) -
`reads_writes_null_terminated_string` and `is_thread_safe` - with an `encode`
helper. The C# file is compiled `#if DEBUG`, so the whole file only exists in
debug builds; every test in it is ported here unconditionally. Two C# cases
have no Rust counterpart, because the Rust `StreamEncoding` enum has no UTF-16
variant: `(Encoding.Unicode, "Hello, ??!")` and the derived
`WriteNullTermString("A", Encoding.Unicode)` -> `[65, 0, 0, 0]` case, and the
"utf16 case where first character byte is null" case, which asserts that
`Encoding.Unicode.GetString(new byte[] { 64, 0, 0, 64 })` has a length of 2.
`Encoding.GetEncoding(0)` and `Encoding.Default` are both UTF-8 on .NET, so
they map to `StreamEncoding::Utf8`.
**Notes and gotchas:** those two cases are not ported; the file header lists
them so that nobody mistakes them for oversights. Together with the empty
`debug_log.rs` and `websocket_uri.rs` facts, they are the only places in the
suite where a C# assertion has no Rust counterpart.

##### `crates/steamkit-tests/tests/web_api.rs` - 649 lines
**C# counterpart:** `SteamKit2/Tests/WebAPIFacts.cs`
**Purpose:** The Web API client - timeouts, the interface factory, the argument
dictionary, the query string and the error mapping.
**How it works:** 12 tests (3 `#[test]`, 9 `#[tokio::test]`) with a local HTTP
server helper: `start`, `base_address`, `next_request`, `read_request`,
`configuration_for` and `interface_for`. The C# suite injects an
`HttpMessageHandler` into the `HttpClient` of the configuration, so that the
request can be inspected without a network round trip. The Rust port has no
such seam, so the base address of the configuration points at a minimal HTTP
server that the test starts on the loopback interface; the server records the
request it receives and answers it. The `find_subslice` and `parse_query`
helpers then inspect the recorded request. Two C# constructs have no direct
Rust counterpart: `WebAPI.Interface` and `WebAPI.AsyncInterface` derive from
`System.Dynamic.DynamicObject`, which allows calls such as
`iface.PerformFooOperation2(args)` - Rust has no dynamic dispatch, so the
interface is called with its explicit methods (`AsyncInterface::call` and
`AsyncInterface::call_with_method`); and the `method` argument of a dynamic
call selects the HTTP method and is not part of the arguments, which is what
`AsyncInterface::call_with_method` does.
**Notes and gotchas:** this is the only test in the suite that starts an HTTP
server, and it does so on the loopback interface. The facts
`uses_args_as_query_string_params`, `supports_null_args_dictionary`,
`uses_single_parameter_arguments_dictionary` and `includes_api_key_in_params`
assert on the recorded query string.

##### `crates/steamkit-tests/tests/websocket_uri.rs` - 73 lines
**C# counterpart:** `SteamKit2/Tests/WebSocketUriFacts.cs`
**Purpose:** Building the `wss://` URI for a CM WebSocket connection.
**How it works:** 4 tests - `dns_end_point`, `ip_end_point_v4`, `ip_end_point_v6`
and `throws_wrong_end_point`. The C# file is wrapped in `#if DEBUG`; Rust has
no such split. The C#
`WebSocketConnection.WebSocketContext.ConstructUri(EndPoint)` accepts an open
ended `EndPoint` and throws an `InvalidOperationException` for an endpoint that
is neither a `DnsEndPoint` nor an `IPEndPoint`. Rust has no open-ended
endpoint type, so the port splits the operation into `construct_uri` (the
`IPEndPoint` case) and `construct_uri_for_host` (the `DnsEndPoint` case).
**Notes and gotchas:** because the type system rejects every other shape, the
"wrong endpoint" fact has **no counterpart** - the error case is expressed as
the absence of a third constructor rather than as a runtime error. The Rust
file still carries a test *named* `throws_wrong_end_point`, and it used to have
an empty body: it only documented that the C# `Assert.Throws<InvalidOperationException>`
for a `DummyEndPoint` cannot be reproduced, because the C# `EndPoint` is an open
ended base class that only the DNS and the IP endpoint derive from, whereas the
Rust `construct_uri` takes a `SocketAddr` and `construct_uri_for_host` takes a
host name. **That test now asserts something** - see 7.2.7. The IPv4 and IPv6
literals matter: the URI has to carry the address in brackets for IPv6
(`wss://[::1]:1337/cmsocket/`).

---

#### 7.2.7 The test-suite audit

An audit of the ported suite was run against the C# tree. It is worth recording
here, because a green suite that cannot fail is not evidence of anything.

**The headline.** **328 Rust test functions** — 262 `#[test]` plus 66
`#[tokio::test]` — against **320 C# cases, of which 319 are active**, because
one C# test sits inside an `#if false` block.

The guide's own count of the C# tree is a little higher, and the difference is
worth recording so that nobody chases it later: the audit's script matched the
`[Fact]` attribute *without* its trailing attribute list, so it counted 246 of
the 255 fact methods and missed the nine facts that `DebugLogFacts.cs` writes
as `[Fact, DebugLogSetupTeardown]`. Counting the attributes by hand gives
**255 `[Fact]` methods** — one of them, `KeyValueFacts.CanReadAndIgnoreConditionals`,
inside `#if false` — and **13 `[Theory]` methods** carrying 72 `[InlineData]`
rows and one `[MemberData]` theory, i.e. 73 theory-derived cases. That is
**328 C# cases, of which 327 are active**. The Rust side of the ledger is the
same 328, and 7.2.5 shows how the two sides reconcile.

**What the audit found.** Five tests that could not fail:

* **Four tests whose bodies contain no assertion at all.** They pass
  unconditionally, so a regression in the code they were written for would not
  be caught:
  * `debug_log.rs::generated_cm_client_id_prefixed`
  * `debug_log.rs::custom_cm_client_id_prefixed`
  * `websocket_uri.rs::throws_wrong_end_point`
  * `steam_user.rs::log_on_throws_exception_if_details_not_provided`
* **One test that looked as if it asserted something but could not.**
  `async_job.rs::async_job_multiple_extends_timeout_on_message` posted a
  result that does not complete the job and then slept **70 ms**, while the
  job manager's timeout is **10 seconds** (`MANAGER_JOB_TIMEOUT`). The
  assertion — "the job is still tracked" — therefore held whether or not the
  result extended the timeout at all.

**How they stand now.** All five have been fixed; the fixes are in the test
files and in one library function, and they are described in the per-file
entries above:

* `debug_log.rs::generated_cm_client_id_prefixed` and
  `custom_cm_client_id_prefixed` now assert what the C# asserts: that the
  category that reaches a `DebugLog` listener is prefixed with the id of the
  client (`"{id}/MyCategory"` and `"My Custom Client/MyCategory"`) and that the
  message is the formatted one. **This required a library change**:
  `CMClient::log_debug` used to log through `tracing` only, so a `DebugLog`
  listener never saw the line. It now builds the C#'s full category, writes the
  line to the global `DebugLog` sink, and emits the `tracing` event next to it.
* `websocket_uri.rs::throws_wrong_end_point` now asserts the behaviour the C#'s
  `ConstructUri` *does* have for the endpoint shapes it supports — always a
  `wss` scheme, always the `/cmsocket/` path, and the host and the port of the
  endpoint — because the C#'s "throw for an unknown endpoint kind" case has no
  Rust counterpart at all.
* `steam_user.rs::log_on_throws_exception_if_details_not_provided` now asserts
  the behaviour that does exist in Rust: `LogOnDetails::default()` carries no
  username, no password and no access token, and `log_on` rejects it with
  `SteamError::InvalidArgument`.
* `async_job.rs::async_job_multiple_extends_timeout_on_message` now waits past
  the manager's **real** deadline — `MANAGER_JOB_TIMEOUT + TIMEOUT_MARGIN`, i.e.
  10.5 seconds — before asserting that the job is still tracked, and it
  separately reproduces the C#'s 50 ms job timeout through the job object's own
  `set_timeout` / `timeout` pair, asserting that `add_result` extends it by
  `JOB_HEARTBEAT_EXTENSION`. The fact is one of the slow ones in the suite, and
  that is the price of it actually testing something.

---

#### 7.2.8 The Rust-only tests of the networking layer and the CM client lifecycle

The ported xunit suite does not test the networking layer at all: there is no
C# test for the TCP or UDP transports, the channel encryption, the event
plumbing, or what `CMClient` does between `Connect` and `Disconnected` - only
`GetPacketMsg`. That is where the defects of 2026-10-22 were (§12 of this
guide), so seven Rust-only test files were added, with **109 tests**. Every
defect they were written for was confirmed by reintroducing it and watching
the test fail.

The CM client and handshake tests do not touch the network. They play the
server through a scripted connection, `tests/mock/mod.rs`, which the client is
handed through its connection factory. The TCP tests use a real socket on the
loopback interface. Nothing reaches Steam.

##### `crates/steamkit-tests/tests/mock/mod.rs` - 197 lines
**C# counterpart:** none.
**Purpose:** A scripted connection - `MockConnection` - that implements
`IConnection` and `ConnectionEvents` without a socket, so that a test can play
the CM server.
**How it works:** A `ConnectBehaviour` picks what `connect` does: `Connected`
(succeeds and raises `Connected` straight away, like TCP), `Silent` (succeeds;
the test raises `Connected`), `Fail`, and `FailWithDisconnected` (fails and also
raises `Disconnected`, which is what the WebSocket transport does). The test
drives the server side with `receive(bytes)`, `raise_connected()` and
`server_disconnect()`, and inspects the client side with `sent()`,
`sent_msg_types()`, `disconnect_calls()`, `connect_count()` and
`subscriber_count()`. `eventually(what, condition)` polls a condition for up to
five seconds; `settle()` waits 100 ms for the tests that assert that something
does *not* happen.
**Notes and gotchas:** it lives in a sub-directory so that cargo does not build
it as a test binary of its own; the test files pull it in with `mod mock;`.

##### `crates/steamkit-tests/tests/connection_events.rs` - 210 lines
**C# counterpart:** none.
**Purpose:** `ConnectionEventSenders`: the ordered stream must deliver every
event, in order.
**How it works:** 9 tests. A burst of 50 x `CONNECTION_EVENT_CAPACITY` events
raised before the subscriber reads any arrives complete and in order; the same
burst makes a broadcast receiver lag, which documents why nothing uses those to
drive a protocol; different event kinds keep their relative order; every
subscriber gets every event; a subscriber sees only the events raised after it
subscribed; dropped subscribers are forgotten; clones share their subscribers;
and four threads raising events at once produce one order that two subscribers
agree on, with each thread's own events in order.
**Notes and gotchas:** the broadcast-lag test is meant to show a `Lagged`
error - it documents the lossy receivers rather than a defect, so do not
"fix" it by raising the capacity. The concurrency test needs the
multi-threaded runtime (`worker_threads = 4`); on the default current-thread
runtime the threads would never interleave and the test would prove nothing.

##### `crates/steamkit-tests/tests/net_filter_encryption.rs` - 225 lines
**C# counterpart:** none.
**Purpose:** `NetFilterEncryptionWithHMAC`, the AES + HMAC filter of every TCP and
UDP CM session.
**How it works:** 13 tests. The wire format is pinned down by an **independent
encoder** built from the suite's own AES-256 and SHA-1 (`tests/common`), which
share no code with the library: packets it builds are decrypted correctly.
Round trips at every length around the block size; the length of the
ciphertext; the caller-buffer form and its too-small buffer; a fresh IV per
packet; a forged HMAC, a flipped bit at **every** offset of a packet, a wrong
key, truncated and misaligned packets, and dropped and appended blocks are all
rejected; and the `Debug` output does not reveal the key.
**Notes and gotchas:** the reference encoder (`reference_encrypt`) must stay
independent of the library - building it from the library's own crypto
helpers would make the test agree with whatever the library does, right or
wrong. The bit-flip test walks every byte of a packet, so keep its message
short.

##### `crates/steamkit-tests/tests/tcp_connection.rs` - 330 lines
**C# counterpart:** none.
**Purpose:** The TCP transport against a real socket.
**How it works:** 14 tests. The `VT01` framing in both directions, a frame
split into single-byte writes, and a burst of 5 000 frames that arrives while
the consumer is busy. `Connected` precedes the first message even when the
server writes the moment the socket is accepted - fifty times over, on a
multi-threaded runtime. A bad magic, an implausible length (without allocating
it) and a frame cut off by the server close the connection; the server closing
it and the user closing it each raise exactly one `Disconnected` with the right
`user_initiated`; sending before connecting and after disconnecting is a silent
no-op; a closed port fails; and one `TcpConnection` can be connected three times.
**Notes and gotchas:** every listener binds `127.0.0.1:0`, so the tests take a
free port each and can run in parallel; a firewall that blocks loopback
connections would fail them. The burst test sleeps 200 ms so the frames pile
up before the consumer reads - that sleep is the point of the test, not a
timing fudge. The `Connected`-first race runs fifty times on the
multi-threaded runtime because a single run rarely hits it.

##### `crates/steamkit-tests/tests/envelope_encrypted_connection.rs` - 421 lines
**C# counterpart:** none.
**Purpose:** The channel-encryption handshake.
**How it works:** 20 tests. The test cannot decrypt the session key - that
takes Valve's private key - but everything up to the switch to the encrypted
state is observable. The `ChannelEncryptResponse` is checked byte by byte (the
header, the protocol version, the key size, the CRC-32 of the encrypted key and
the trailing zero) and carries a fresh key every time. A request that arrives
right behind `Connected` is answered, fifty times; a request is answered in
every universe; `Connected` is raised only once the result arrives; a failed
result, a short challenge, a challenge too long for RSA-1024 OAEP, a truncated
request, an unknown universe and an unparseable message disconnect; a request
for another universe is still answered, as in the C#; unexpected messages
during the setup are ignored; messages go out in the clear before the
handshake and encrypted after it; a packet that does not decrypt is dropped;
the inner disconnection is passed on and resets the handshake, which then works
again on the same object; the forwarding task ends when the connection is
dropped; and the debug listener sees the three handshake messages.
**Notes and gotchas:** a test that asserts something does *not* happen
(ignored messages, no second response) waits with `settle()`, so it can only
catch a regression that shows up within 100 ms. Positive assertions use
`eventually(..)` and never a fixed sleep. Because the session key cannot be
decrypted, the encrypted path is checked by shape (the length and that the
bytes differ from the plain text), not by content.

**C# counterpart:** none (`CMClientFacts` covers only `GetPacketMsg`; see `cm_client.rs`).
**Purpose:** What `CMClient` does between `connect` and the disconnection.
**How it works:** 27 tests, each with a `CMClient` whose connection factory hands
out `MockConnection`s and whose dispatcher records what the client reports.
*Dispatching* - the hello; a burst of 2 000 messages behind a slow handler,
complete and in order; a message right behind `Connected` is dispatched after
it; an unparseable packet and `ClientServerUnavailable` disconnect; the session
token is recorded. *Multi messages* - plain and gzipped, nested, a malformed
message inside one (which disconnects, as in the C#), and one that does not
decompress. *Logging on* - the identity from the logon response is stamped on
outgoing messages; the heartbeat starts; a log off clears the identity. *The
server closing the connection* - before a logon it is reported once and not
resumed; after a logon the session moves to another server and repeats the
same logon bytes after the hello, without a disconnection; a refused logon, a
`ClientLoggedOff` and an expected disconnection are **not** resumed; a failed
move is reported once; the moves give up after
the session ends and the caller reconnects.
that limit. *The server asking for another server* - `TryAnotherCM` and
`ServiceUnavailable` move and repeat the logon. *Disconnecting* - a user
disconnect is reported once and the connection is no longer listened to; a
failed connection attempt is reported once for both kinds of failure; and a
second `connect` ends the previous session once, does not repeat its logon,
and cannot be disturbed by the old connection.
**Notes and gotchas:** the recorder's `slow_first_message` flag makes the
dispatcher sleep 200 ms on the first message only, which is how the 2 000-message
burst is made to queue up behind it. A test that moves to another server must
wait for the next `MockConnection` (`connection_count()`) before driving it,
because the factory hands it out asynchronously. The "is not resumed" tests rely
on `settle()`, with the same 100 ms limit as above.

##### `crates/steamkit-tests/tests/udp_packet.rs` - 221 lines
**C# counterpart:** none.
**Purpose:** The UDP datagram format.
**How it works:** 16 tests. The 36-byte header's documented little-endian layout
(the magic reads `VS01`) and its round trip, every packet type, an unknown type,
the C# defaults, and every truncation; packet round trips, the payload limit of
`MAX_PAYLOAD`, payload slices past the end of the source, a bad magic, a short
datagram, an oversized payload size and a datagram shorter than its payload
size (zero-padded, as the C# does); and the `ChallengeData` / `ConnectData`
handshake payloads.
**Notes and gotchas:** the zero-padding of short datagrams and of payload
slices past the end is the C# behaviour, kept on purpose. It looks like a
missing bounds check, but rejecting such packets would diverge from the
original. These tests are synchronous and touch no socket.

##### `crates/steamkit-tests/tests/msg_util.rs` - 91 lines
**C# counterpart:** none.
**Purpose:** `MsgUtil`, the protobuf flag on message numbers, and
`KeyDictionary`, the universe public keys.
**How it works:** 10 tests. Setting, stripping and testing the protobuf flag,
for `EMsg` and for game-coordinator numbers; the free functions match the
associated ones; every real universe has a distinct DER key and `Invalid` has
none; and the handshake's `public_key_for_universe` agrees with
`KeyDictionary` - the test that found the corrupt `Dev` key.
**Notes and gotchas:** the handshake keeps its own key lookup
(`public_key_for_universe`) next to `KeyDictionary`, so the two can drift
apart; the agreement test is what keeps them in step, and it must cover every
universe, `Dev` included, since that is the one that was wrong.

---

### 7.3 What the suite found

The suite was ported first and compiled last, because the workers were told not
to run cargo. Compiling it exposed a list of defects that "it compiles" had
hidden. Every one of them was a **silent wrong-answer bug**, not a compile
error - the code built cleanly and produced the wrong bytes.

§12 of this guide records them:

| # | Where | Defect | Fix |
|---|---|---|---|
| 1 | `util/crypto_helper.rs` | `symmetric_decrypt` allocated a zeroed buffer and never copied the ciphertext in - it had never decrypted anything | `buffer.copy_from_slice(ciphertext)` |
| 2 | `steam/cdn/depot_chunk.rs` | the same zeroed buffer, and the IV was decrypted from the zeroed `iv` instead of `data[..16]` | decrypt `data[..16]`, use `ciphertext.to_vec()` |
| 3 | `util/lzma_decoder.rs` | `bit - 1` on a `u32` panicked; the C# computes it on a `uint`, which wraps | `bit.wrapping_sub(1)` |
| 4 | `types/depot_manifest.rs` | `new_from_slices` had key and IV reversed; strict base64 rejected the trailing newline the C# ignores; the null trim checked the byte *past* the plaintext | all three corrected |
| 5 | `base/generated/steam_language_internal.rs` | the wire macro captured the field kind as `$kind:ty` and tried to re-match it against literal tokens - a `ty` fragment is opaque, so it could never work | a `WireValue` trait; the field's own type carries the encoding |
| 6 | `steam/discovery/smart_cm_server_list.rs` | the HTTP client factory was invoked while the configuration was being built; the C# invokes it when the server list is actually fetched | the client is built on first fetch, in a `OnceLock` |
| 7 | `types/key_value.rs` | the text reader only stripped a UTF-8 BOM; the C# `StreamReader` also detects UTF-16 LE and BE | BOM sniffing plus a UTF-16 decoder |
| 8 | `types/steam_id.rs` | `render(false)` fell back to the Steam3 form for every account type but `Invalid`/`Individual`; the C#'s `Convert.ToString(this)` binds to the **`ulong`** overload, so it renders the numeric id | render the `u64` |
| 9 | `util/lzma_util.rs` | `try_decompress` handed `decompress` a *pre-filled* buffer, and a `Vec` that is written to grows at its end, so the decoded data landed at an offset; the C# writes into a `MemoryStream` that has the capacity but no content | empty the buffer before decompressing |
| 10 | `util/lzma/compress/lzma/lzma_encoder.rs` | `baseVal - posSlot - 1` underflowed for `posSlot == 4`; the C# computes it on a `UInt32`, which wraps to `0xFFFFFFFF`, and its bit tree then wraps the index back to `0` | `wrapping_sub`, and `wrapping_add` for the index inside the bit tree |

Two more defects were found by running the ported **tools** and **samples**
rather than the library tests:

| # | Where | Defect | Fix |
|---|---|---|---|
| 11 | `steam/cm_client.rs` | `default_connection_factory` never used the WebSocket transport that the port already ships, and its order was TCP/UDP/WebSocket instead of the C#'s WebSocket/TCP/UDP. A WebSocket-only server candidate - which is what the default fallback list holds first - could never connect | the transport is wired up in the C#'s order, and the host name is plumbed through so the websocket URI carries the TLS server name |
| 12 | `steamkit-tools/nethook/message_reading.rs` | the reader stripped the message number and then let the header read it again; the C#'s `MsgHdrProtoBuf` carries the message number itself, so the analyzer could not decode a single real capture | the protobuf header is read from the start of the capture |

The counts in the repository's prose vary with how the two related LZMA fixes
are grouped: §12 of this guide says "nine real defects" over a ten-row table, and
§12 of this guide lists the twelve defects over eleven numbered subsections
(§4.1-§4.11, with the LZMA encoder fix folded into §4.9). The table above is
the precise record: **ten defects from the test suite, two more from the tools
and the samples.**

**The lesson, recorded so it is not repeated:** a port that only compiles is not
a port that works. The suite has to be compiled *and* run, and the binaries have
to be executed, before "done" means anything.

### 7.4 Running the suite

```powershell
cd <workspace root>
$env:CARGO_BUILD_JOBS = 2
cargo test --workspace --no-fail-fast
```

Expect **458 passed, 0 failed, 4 ignored**. `--no-fail-fast` matters: it lets
every test binary report, so one failure does not hide the rest. §11.1 of this guide
governs the build (one shared `target/`, `CARGO_BUILD_JOBS=2`, never two cargo
runs at once).

If anything fails, treat it as a new bug of the §7.3 class: investigate the
**wire format**, not the test.

#### The live depot test

##### `crates/steamkit-tests/tests/depot_e2e.rs` - 318 lines
**C# counterpart:** none - a new live test for this port.
**Purpose:** A live, network-dependent end-to-end test of the whole depot path: it logs in
to Steam anonymously over the CM connection, asks the CM server for the depot decryption
key, asks the content server directory for a CDN auth token, downloads the depot manifest
and its chunks, decrypts them, and compares the result with a reference file produced by
the C# SteamKit2.
**How it works:** Every parameter comes from the environment - `STEAM_APP`, `STEAM_DEPOT`,
`STEAM_MANIFEST`, `STEAM_FILE` and the optional `STEAM_EXPECT` path of the reference
file. The test is `#[ignore]`d, because it needs the network; run it with
`cargo test --test depot_e2e -- --ignored --nocapture`.
**Notes and gotchas:** The anonymous logon must happen before any job is requested - the
CM server refuses jobs from a session that has not logged on yet, and answers with
`EMsg.DestJobFailed`. The depot key itself is ownership-gated: both this port and the C#
reference are refused it for an account that does not own the app, so the test needs an
account that does.

---

## 8. `crates/steamkit-nethook/` — the NetHook2 DLL


### 8.1 Overview


#### 8.1 What NetHook2 is for

NetHook2 is a Windows DLL that is **injected into the address space of a running
`Steam.exe`** in order to observe everything the Steam client sends to, and receives
from, the Steam servers. It does three things:

1. It patches three functions inside the Steam client library (`steamclient64.dll`,
   or `steamclient.dll` on 32-bit Windows):
   - `CWebSocketConnection::BBuildAndAsyncSendFrame` — every binary WebSocket frame
     the client sends;
   - `CCMInterface::RecvPkt` — every packet the client receives;
   - `CCrypto::SymmetricEncryptChosenIV` — the *plaintext* of every message the
     client encrypts, captured before encryption.
2. It writes each captured message to its own file on disk.
3. `NetHookAnalyzer2` (the C# WPF GUI) — ported to Rust as the
   `steamkit-nethook-analyzer` binary — reads that directory of files and decodes
   them.

The DLL and the analyzer are two halves of one tool: the **sniffer** produces the
capture files, the **analyzer** consumes them. The contract between them is the
file name.

#### 8.2 The C++ original and what the port changed

The C# tree contains NetHook2 as a **C++** Visual Studio project under
`Resources/NetHook2/NetHook2/`: **12 `.cpp` + 19 `.h`**, roughly **13.7k lines**
(the bulk of it is the checked-in copy of Valve's generated
`steammessages_base.pb.{h,cc}`, ~19.5k lines of which the C++ never used as
protobuf at runtime beyond `CMsgMulti`/`CMsgProtoBufHeader`). It was built with
MSVC, depended on **Microsoft Detours** (pulled in through vcpkg) for its function
hooking, and on zlib for gzip decompression.

The Rust port (`crates/steamkit-nethook/`, 22 `.rs` files + `Cargo.toml` +
`build.rs`) changed three things materially:

| Concern | C++ | Rust port |
|---|---|---|
| Function hooking | Microsoft **Detours** (`DetourTransactionBegin` / `DetourAttach` / `DetourDetach`) — an external vcpkg dependency | A **hand-written inline trampoline patcher** in `src/csimpledetour.rs`: the prologue is decoded instruction by instruction, relocated into a freshly allocated executable trampoline, and the target is patched with an absolute jump. Detours' thread-suspension behaviour is reproduced by suspending every other thread of the process while the patch is applied. |
| Protobuf | A checked-in copy of Valve's `steammessages_base.pb.{h,cc}` | The messages are **already generated** into `steamkit-protos` by `prost`; the crate consumes `steamkit_protos::steam::*` / `steamkit_protos::gc::*` and the generated C++ files are not ported at all. |
| Gzip | zlib `inflateInit2(&zstrm, 16)` (gzip-only window) | `flate2::read::GzDecoder`, with the same "the stream must end exactly at the declared size" contract. |

Two smaller deviations: the C++ `__fastcall`/`__thiscall` member-function
signatures have no 64-bit equivalent, so the Rust hooks use the system ABI
(`extern "system"`); and the C++ `SelfInjectIntoSteam` loaded the DLL by ANSI
path (`LoadLibraryA`), while the Rust port hands the path over as UTF-16 and
loads it with `LoadLibraryW`.

#### 8.3 The crate is Windows-only and is the only `unsafe` in the workspace

`src/lib.rs` opens with

```rust
#![cfg(windows)]
```

so the **entire crate** is compiled out on any non-Windows target. That is what
keeps `cargo check --workspace` working on a Linux or macOS checkout: the crate
still exists in the member list, but contributes no code. Every module reaches
the operating system through the `windows` crate, which only exists for Windows
targets.

This crate is also the **only place in the workspace that uses `unsafe`**. The
library crate `crates/steamkit` declares `#![forbid(unsafe_code)]`
(`crates/steamkit/src/lib.rs:23`), and so do the other crates by inheritance from
that policy; `steamkit-nethook` is the single deliberate exception, because
hooking a function inside another process is raw-pointer work by nature. (Note
that §3.4 of this guide describes the error model as "scopes its own
`#![allow(unsafe_code)]`" — the crate in fact carries **neither**
`#![forbid(unsafe_code)` nor `#![allow(unsafe_code)]`; it simply omits the lint
attribute and uses `unsafe` freely, one block at a time, each with a `SAFETY:`
comment. The substance of the policy — one crate, one exception, every `unsafe`
block justified in place — holds.)

#### 8.4 The on-disk contract with the analyzer

Every captured message becomes **one file**. The name is built from the C++
format string `"%03u_%s_%d_%s"` (`CLogger::GetFileNameBase`):

```
{sequence:03}_{direction}_{emsg}_{message name}.bin
```

| Part | Meaning | Example |
|---|---|---|
| `%03u` | zero-padded to three digits, at least; the counter starts at **1** and increments for every captured message | `001` |
| `%s` | `in` (received from the server) or `out` (sent to the server) | `out` |
| `%d` | the numeric `EMsg` of the message | `700` |
| `%s` | the human-readable message name, taken from Steam's own `PchMsgNameFromEMsg` | `ClientLogon` |

The extension is **not** part of the format string. The logger writes the
payload to `<name>.tmp` and then renames it to `<name>.bin`, so a partially
written capture never appears with its final name.

**Only the first three parts are contractual.** The analyzer finds them with the
regular expression

```
(?<num>\d+)_(?<direction>in|out)_(?<emsg>\d+)
```

which is exactly the C# `NetHookItem.cs` `[GeneratedRegex]` (with
`RegexOptions.IgnoreCase`). The message name and the `.bin` extension are
ignored by the parser. The Rust analyzer
(`crates/steamkit-tools/src/nethook/item.rs`, function
`parse_capture_file_name`) reimplements that pattern **by hand** — leftmost
match wins, case-insensitive direction, the rest of the name discarded — rather
than pulling in a regex dependency.

Files land in:

```
<directory of the host process's main module>\nethook\<unix timestamp>\
```

For an injected DLL that is `<Steam install dir>\nethook\<epoch seconds>\`, one
session directory per injection. The session directory name is the Unix time
in seconds at the moment the logger was constructed.

#### 8.5 What has and has not been verified

Stated plainly, because this distinction matters:

- **Verified:** the crate compiles as a `cdylib` on Windows.
- **Verified:** the capture format the DLL writes is **provably consistent with
  the analyzer**. A capture produced by this port was decoded by the port's own
  `steamkit-nethook-analyzer` (`crates/steamkit-tools/src/bin/steamkit-nethook-analyzer.rs`).
  That is the proof that the format contract holds end to end.
- **Not verified:** the DLL has **never been injected into a real `Steam.exe`**.
  Neither the injector (`src/injector.rs` — remote `LoadLibraryW` via
  `CreateRemoteThread`) nor the trampoline patcher (`src/csimpledetour.rs`) has
  ever run against a live Steam process. The byte patterns in `src/net.rs` and
  `src/crypto.rs` are also untested against a real `steamclient64.dll` build.

Everything in this section that describes runtime behaviour is therefore a
**faithful port of the C++**, not a report of observed behaviour.

#### 8.6 File map

| Rust file | C++ counterpart |
|---|---|
| `src/lib.rs` | crate root; maps the whole project |
| `src/nethook.rs` | `NetHook2/nethook.cpp` |
| `src/injector.rs` | `NetHook2/injector.cpp` |
| `src/logger.rs` | `NetHook2/logger.cpp`, `NetHook2/logger.h` |
| `src/net.rs` | `NetHook2/net.cpp`, `NetHook2/net.h` |
| `src/crypto.rs` | `NetHook2/crypto.cpp`, `NetHook2/crypto.h` |
| `src/csimpledetour.rs` | `NetHook2/csimpledetour.cpp`, `NetHook2/csimpledetour.h` (plus the Detours library itself) |
| `src/csimplescan.rs` | `NetHook2/csimplescan.cpp`, `NetHook2/csimplescan.h` |
| `src/sigscan.rs` | `NetHook2/sigscan.cpp`, `NetHook2/sigscan.h` |
| `src/sedebug.rs` | `NetHook2/sedebug.cpp`, `NetHook2/sedebug.h` |
| `src/binaryreader.rs` | `NetHook2/binaryreader.cpp`, `NetHook2/binaryreader.h` |
| `src/nh2_string.rs` | `NetHook2/string.cpp`, `NetHook2/nh2_string.h` |
| `src/steamclient.rs` | `NetHook2/steamclient.h` |
| `src/version.rs` | `NetHook2/version.h` (+ the generated `version.cpp`) |
| `src/zip.rs` | `NetHook2/zip.cpp`, `NetHook2/zip.h` |
| `src/steam/mod.rs` | `NetHook2/steam/` (the directory) |
| `src/steam/emsg.rs` | `NetHook2/steam/emsg.h` (reduced) |
| `src/steam/net.rs` | `NetHook2/steam/net.h` |
| `src/steam/clientmsgs.rs` | `NetHook2/steam/clientmsgs.h` |
| `src/steam/csteamid.rs` | `NetHook2/steam/csteamid.h` |
| `src/steam/steamtypes.rs` | `NetHook2/steam/steamtypes.h` |
| `src/steam/udppkt.rs` | `NetHook2/steam/udppkt.h` (**not compiled** — see §8.23) |

#### `crates/steamkit-nethook/Cargo.toml` — 43 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/NetHook2.vcxproj` (the MSVC project file)
**Purpose:** Declares the crate as a Windows DLL and pins the three dependency families it needs: the generated protobuf messages, a gzip decoder, and the Win32 API surface.
**How it works:** The `[lib]` section sets `name = "steamkit_nethook"` and **`crate-type = ["cdylib"]`** — the crate produces a `.dll` with no Rust `rlib` companion, which is what `rundll32` and `LoadLibrary` need. Dependencies are three: `steamkit-protos` (path dependency, the `prost`-generated Valve messages) and `prost` itself for the `Message` trait that `CMsgMulti::decode` needs; `flate2` for the gzip decode that replaces zlib's `inflateInit2(&zstrm, 16)`; and `windows` 0.62 with an explicit feature list — `Win32_Foundation`, `Win32_Security`, `Win32_Storage_FileSystem`, `Win32_System_Console`, `Win32_System_Diagnostics_Debug` (for `FlushInstructionCache`), `Win32_System_Diagnostics_ToolHelp` (for the thread snapshot the detour takes), `Win32_System_IO`, `Win32_System_LibraryLoader`, `Win32_System_Memory`, `Win32_System_ProcessStatus`, `Win32_System_SystemInformation`, `Win32_System_SystemServices` (the `DLL_PROCESS_ATTACH`/`DETACH` constants), `Win32_System_Threading`, `Win32_UI_WindowsAndMessaging`. The single build dependency is `chrono`, used only by `build.rs` to stamp the build date.
**Notes and gotchas:** There is **no `rlib`** — nothing in the workspace can `use steamkit_nethook::…` as a library; the DLL is a leaf artifact. The `windows` feature list is deliberately narrow: each missing feature is a compile error, not a runtime one, so the list is the authoritative statement of which Win32 APIs this crate may call. The C++'s `steammessages_base.pb.{h,cc}` are *not* ported — the comment in the manifest states the reason: the same messages are already generated by `prost` into `steamkit-protos`.

#### `crates/steamkit-nethook/build.rs` — 39 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/GenerateVersionInfo.ps1`
**Purpose:** Stamps four build-time values into the DLL — the build date, the short commit SHA, the commit date, and whether the working tree was dirty.
**How it works:** The C++ project ran a PowerShell script that *generated a source file* (`version.cpp`) holding four globals. Rust cannot generate a source file at build time, so the port exports the same four values through `cargo:rustc-env` instead, which `src/version.rs` reads back with `option_env!`. The script runs `git rev-parse --short HEAD` and `git show -s --format=%ci HEAD` for the SHA and commit date, and `git diff --quiet` for the dirty flag — a non-zero exit from `git diff --quiet` means "the tree has changes". The build date is `chrono::Local::now().format("%Y-%m-%d %H:%M:%S %:z")`. Every value is emitted as `cargo:rustc-env=NETHOOK_…`.
**Notes and gotchas:** Every git call is **optional**: `git()` returns `None` if the binary is missing or the command fails, and `unwrap_or_default()` turns that into an empty string — matching the C++ script, which left the SHA and commit date empty when git was unavailable. `build.rs` does **not** set `cargo:rerun-if-changed` on the source tree, only on itself, so the stamped values do not change when the working tree changes; they are fixed until `build.rs` itself is touched or the crate is cleaned. The C++ additionally stamped a Windows `VERSIONINFO` resource into the binary via a resource script — that step is a resource-script concern and is **not** reproduced, so the DLL has no version resource.

#### `crates/steamkit-nethook/src/lib.rs` — 34 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/NetHook2.vcxproj` (project definition) and `Resources/NetHook2/NetHook2/nethook.cpp` (the DLL entry point it re-exports)
**Purpose:** Crate root. Gates the whole crate to Windows, declares every module, and documents the one-to-one mapping between the C++ files and the Rust modules.
**How it works:** The module table in the crate-level doc comment maps each C++ source file to its Rust counterpart — `nethook.cpp`→`nethook`, `injector.cpp`→`injector`, `net.{cpp,h}`→`net`, `crypto.{cpp,h}`→`crypto`, `logger.{cpp,h}`→`logger`, `csimplescan`→`csimplescan`, `csimpledetour`→`csimpledetour`, `sigscan`→`sigscan`, `sedebug`→`sedebug`, `binaryreader`→`binaryreader`, `string.cpp`/`nh2_string.h`→`nh2_string`, `zip`→`zip`, `version.h`→`version`, `steamclient.h`→`steamclient`, `steam/*.h`→`steam`. Fifteen modules are declared `pub`, and two re-exports are surfaced at the crate root: `pub use injector::{Eject, Inject};` and `pub use nethook::DllMain;`. The doc comment also records that the exported entry points are the same two as the C++ (`Inject` and `Eject`) and that `DllMain` is the entry point, so the DLL is driven with `rundll32 "<path>",Inject` exactly like the original.
**How it works (continued):** Immediately before the module list sits the single most important line of the file:

```rust
#![cfg(windows)]
```

This is a **crate-level** `cfg`, not a per-module one: on a non-Windows target the entire crate body — every `pub mod` line included — is discarded before name resolution, so `cargo check --workspace` succeeds on Linux or macOS without the `windows` crate being buildable there.
**Notes and gotchas:** The C++ DLL exports `Inject` and `Eject` — **not** `Uninject`; the Rust names match the C++ exactly. The generated `steammessages_base.pb.{h,cc}` are explicitly *not* ported, per the doc comment, because their message types are already present in `steamkit-protos`. The crate carries **no** `#![forbid(unsafe_code)]` and **no** `#![allow(unsafe_code)]`; it is the workspace's deliberate `unsafe` exception and simply uses `unsafe` blocks locally.

#### `crates/steamkit-nethook/src/nethook.rs` — 109 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/nethook.cpp`
**Purpose:** The DLL entry point. It decides whether the DLL is being loaded into `rundll32.exe` (the injector half) or into Steam (the hook half), and in the latter case allocates a console, makes sure the Steam client library is loaded, and installs the crypto and network hooks.
**How it works:** `DllMain` is `#[no_mangle] pub extern "system" fn DllMain(hinst_dll: HINSTANCE, fdw_reason: u32, _lpv_reserved: *mut c_void) -> i32`. It first calls `is_run_dll32()`, which reads the **host process's** main module path with `GetModuleFileNameA(None, …)` and tests whether it ends with `\rundll32.exe` (case-insensitively). If it does, `DllMain` returns `1` immediately and installs nothing — this is the guard that stops the injector half from hooking `rundll32` itself. Otherwise it branches on `fdw_reason`:

- `DLL_PROCESS_ATTACH` — `AllocConsole()` (success recorded in the `OWNS_CONSOLE` atomic), then `LoadLibraryA(STEAMCLIENT_DLL)` to make sure the Steam client library is mapped (the signature scans need it), then `init_logger()`, `print_version_info()`, `init_crypto()`, `init_net()`.
- `DLL_PROCESS_DETACH` — `shutdown_net()`, `shutdown_crypto()`, then `FreeConsole()` **only** if this DLL allocated it.

The C++ order is identical (AllocConsole → LoadLibrary → `new CLogger` → `PrintVersionInfo` → `new CCrypto` → `new CNet`), and the C++ `DllMain` deleted the three globals in the same order on detach.

`print_version_info()` writes `"Initializing NetHook2..."` and then either `"Built at <date>. No further build information available."` (when the commit SHA is empty) or `"Built at <date> from <sha>"`, optionally followed by `"/dirty"` and `" (<commit date>)"`.
**Notes and gotchas:** The `is_run_dll32()` check is the **first** thing that runs, before the `fdw_reason` match — a bug here would install the hooks inside `rundll32` and hook the wrong process. The `AllocConsole` result is remembered in `static OWNS_CONSOLE: AtomicBool` so that detach only frees a console it actually created; the C++ did the same with `g_bOwnsConsole`. The C++ called `GOOGLE_PROTOBUF_VERIFY_VERSION` here — irrelevant for `prost`, so it is gone. The Rust `LoadLibraryA` takes a `PCSTR` built from `STEAMCLIENT_DLL.as_ptr()`; `STEAMCLIENT_DLL` is a `&'static str` whose NUL terminator is the one Rust guarantees for string literals, so the pointer is already NUL-terminated. The `unsafe` blocks here are `GetModuleFileNameA`, `AllocConsole`, `LoadLibraryA` and `FreeConsole` — each is a direct, well-understood Win32 call with no raw-pointer arithmetic.

#### `crates/steamkit-nethook/src/injector.rs` — 1103 lines
**The injector is now exercised.** The crate carries sixteen tests that run the injector's
enumeration against the machine they run on: `find_process_by_name` finds the running test
binary and reports a name that cannot exist as missing, `process_has_module_loaded` finds a
module that is certainly loaded and rejects one that is certainly not - and, importantly,
does not match a module whose path is only a prefix of a loaded one - `module_path_of`
returns the path of the current executable, `process_is_different_architecture` reports the
same architecture for the current process and for one it cannot open, `get_steam_process_id`
reports a missing Steam client, and `try_parse_int` accepts a plain number while rejecting a
non-number, an empty string and a number that does not fit into an `i32`.

That does not prove the hooking works - that needs a live `Steam.exe`, and the Steam client
is not installed on the machine the port was developed on. What it does prove is that every
step the injector takes *before* the hooking resolves a module, reads a process and parses
its command line actually works, so the failure surface in the part that has never run is
now limited to the trampoline itself.

**C# counterpart:** `Resources/NetHook2/NetHook2/injector.cpp`
**Purpose:** The half of NetHook2 that runs **outside** the Steam process. It is driven by `rundll32` and either injects the DLL into Steam or ejects it again.
**How it works:** Two exported functions with the exact `rundll32` signature `extern "system" fn(HWND, HINSTANCE, *mut c_char, i32)`:

```
rundll32.exe C:\Path\To\NetHook2.dll,Inject
rundll32.exe C:\Path\To\NetHook2.dll,Inject <process ID>
rundll32.exe C:\Path\To\NetHook2.dll,Inject <process name>
rundll32.exe C:\Path\To\NetHook2.dll,Eject
```

Both `Inject` and `Eject` follow the same shape:

1. Parse the command line (`rundll32` passes everything after the comma).
2. `get_steam_process_id` — empty command line → `find_steam_process_id()` (the single process whose executable is `steam.exe`); a decimal string → that PID; anything else → `find_process_by_name` (matching the executable path's `\name` suffix, case-insensitively). Then the target must have the same architecture as this DLL (`IsWow64Process2` on both handles) and must have `steamclient64.dll` loaded.
3. If the failure was one of the three "could not see the process" errors, acquire `SeDebugPrivilege` and retry the search — Steam sets its own DACL so that `PROCESS_VM_READ` is denied without it.
4. `Inject` refuses if this DLL's own path is already loaded in the target; `Eject` refuses if it is not.
5. `Inject` → `self_inject_into_steam`; `Eject` → `inject_ejection`.

`self_inject_into_steam` opens the target with `PROCESS_ALL_ACCESS`, resolves `kernel32!LoadLibraryW` via `GetModuleHandleA`/`GetProcAddress`, allocates memory **in the target** with `VirtualAllocEx(MEM_RESERVE|MEM_COMMIT, PAGE_READWRITE)`, writes the NUL-terminated **UTF-16** DLL path into it with `WriteProcessMemory`, and starts a remote thread at `LoadLibraryW` with that pointer as the argument (`CreateRemoteThread`), waiting up to 10 s. Every allocation and every handle is released on every path.

`inject_ejection` is the interesting one. The C++ copied 0x1000 bytes of its own `_Eject` function into the target and started it there, because "this seems to be the only way to not crash Steam when ejecting". A Rust function cannot be copied that way — it is not position independent and it references the Rust runtime — so the port **hand-assembles the equivalent machine code** in `const EJECT_CODE: &[u8]`:

- **x86-64** (24 bytes): `push rbx` · `sub rsp,0x20` · `mov rbx,rcx` · `mov rcx,rbx` · `add rcx,0x10` · `call qword ptr [rbx]` · `mov rcx,rax` · `call qword ptr [rbx+8]` · `add rsp,0x20` · `pop rbx` · `xor eax,eax` · `ret`.
- **x86** (23 bytes): `mov eax,[esp+4]` · `lea ecx,[eax+8]` · `push ecx` · `call dword ptr [eax]` · `push eax` · `mov edx,[esp+8]` · `push eax` · `call dword ptr [edx+4]` · `add esp,4` · `xor eax,eax` · `ret`.

The code takes the address of a parameter block in its first argument and calls the two function pointers stored in it. The parameter block is laid out as `[GetModuleHandleW pointer][FreeLibrary pointer][UTF-16 module name]`, with the name at offset `eject_params_name_offset()` — 16 on x86-64, 8 on x86 — so the name stays two-byte aligned for `GetModuleHandleW`. The block is written into the target with `WriteProcessMemory`, the code is written into a second `VirtualAllocEx(MEM_COMMIT, PAGE_EXECUTE_READWRITE)` region, and a remote thread is started at the code with the block as its argument (5 s timeout).

Errors are reported to the user with `MessageBoxA` (title `"NetHook2"`, `MB_OK | MB_ICONASTERISK`), exactly as in the C++. The search failures have their own enum `SteamProcessSearchError` and the string table `name_from_steam_process_search_error` reproduces the C++ `NameFromESteamProcessSearchError` verbatim.
**Notes and gotchas:** Three real differences from the C++:

- **UTF-16 instead of ANSI.** The C++ used `LoadLibraryA` and an ANSI path; the Rust port uses `LoadLibraryW` and `GetModuleFileNameW`, so install paths the ANSI code page cannot represent now work. The *eject* path is UTF-16 too, which is why the hand-assembled code calls `GetModuleHandleW` rather than the C++'s `GetModuleHandleA`.
- **A C++ bug is fixed.** The C++ `SelfInjectIntoSteam` allocated `strlen(szNetHookDllPath)` bytes and wrote `strlen(szNetHookDllPath)` bytes — the path was **not** NUL-terminated in the target, so `LoadLibraryA` read past the end of the allocation. The Rust port allocates `(len + 1) * 2` bytes and writes the terminating NUL.
- **`TryParseInt` semantics differ slightly.** The C++ used `isdigit` (locale-dependent) plus `atoi`; the Rust uses an explicit ASCII-digit test and `str::parse::<i32>()`, so an empty string is rejected rather than parsed as zero.

Further gotchas: `find_process_by_name` enumerates at most 2048 processes and opens each with `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`; `process_has_module_loaded` enumerates at most 1024 modules and compares either by exact path (`_stricmp` equivalent) or by path suffix. `needs_debug_privilege` returns `true` for exactly the three errors the C++ retried on (`CouldNotFindSteam`, `CouldNotFindProcessWithSuppliedName`, `TargetProcessDoesNotHaveSteamClientDllLoaded`). `Inject` computes the ANSI path (`own_module_path`) for the "already injected" check and the wide path (`own_module_path_wide`) for the actual injection — it calls `own_module_path_wide` a second time rather than reusing a value. The `Eject` export is named `Eject`, matching the C++ and the `rundll32` invocation above. **This whole module is the least-proven part of the port**: it has never run against a live Steam process.

#### `crates/steamkit-nethook/src/logger.rs` — 366 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/logger.cpp` and `Resources/NetHook2/NetHook2/logger.h`
**Purpose:** Writes the captured traffic to disk as one file per message, and produces the file names that the analyzer parses. It also owns the console output the DLL prints to.
**How it works:** The global logger is a `OnceLock<Logger>`; `init_logger()` constructs it during `DLL_PROCESS_ATTACH` and `logger()` returns `&'static Logger` (it `expect`s, on the grounds that the logger exists before any message can be captured). `Logger` holds `root_dir: Vec<u8>`, `log_dir: Vec<u8>` and `msg_num: AtomicU32`.

`Logger::new()` computes the root directory from `GetModuleFileNameA(None, …)` — the **host process's** main module, i.e. `Steam.exe` when injected — truncating at the last `\`, then appends `\nethook\`. `CreateDirectoryA` creates it. The session directory is the root plus the current Unix time in seconds plus `\`; `CreateDirectoryA` creates that too. Both paths are kept as byte buffers, and `nul_terminated()` produces a NUL-terminated copy for the ANSI Win32 calls.

`capture_file_name(sequence, direction, emsg, name)` is the whole format contract in one function:

```rust
format!("{:03}_{}_{}_{}", sequence, direction.tag(), emsg, name)
```

`NetDirection::tag()` returns `"in"` or `"out"`. A unit test, `capture_file_name_matches_the_c_format_string`, pins the four cases `001_in_2_ProtoBuf`, `001_out_65535_ProtoBuf`, `042_in_0_Null` and `1000_out_1_Name` — note that `{:03}` pads to *at least* three digits and does not truncate, exactly like `%03u`.

The capture path is `log_net_message(direction, data)`:

1. Fewer than two bytes → return (the C++ read the message number out of bounds here).
2. The message number is the **little-endian `u16`** at `data[0..2]` — `EMsg::from_raw(u16::from_le_bytes([data[0], data[1]]) as u32)`.
3. If it is `EMsg::MULTI` (value 1) → `multiplex_multi(direction, data)`; otherwise → `log_session_data(direction, data)`.

`log_session_data` builds the base name via `file_name_base(direction, emsg, 0xFF)`, writes the payload to `<name>.tmp` with `CreateFileA(CREATE_ALWAYS)`, closes the handle, and renames it to `<name>.bin` with `MoveFileA`. It then logs `"Wrote <n> bytes to <name>"` to the console.

`file_name_base` is the exact counterpart of `CLogger::GetFileNameBase`:

```rust
let sequence = self.msg_num.fetch_add(1, Ordering::Relaxed) + 1;
let name = message_name(emsg, server_type).unwrap_or("(null)");
capture_file_name(sequence, direction, emsg.raw(), name)
```

The counter therefore starts at **1** (the C++ used `++m_uiMsgNum`). The `"(null)"` fallback reproduces what the C++ `printf("%s", …)` did when `CCrypto::GetMessage` returned a null pointer.

`multiplex_multi` handles Valve's multiplexed batch format:

1. The first eight bytes are the `ProtoHdr`: an `i32` message number and an `i32` header length, both little-endian.
2. A `CMsgProtoBufHeader` protobuf is decoded from `data[8..8+header_length]` (the decode result is discarded — the C++ parsed it only to log).
3. A `CMsgMulti` protobuf follows, at `data[8+header_length..]`. Its `size_unzipped` and `message_body` fields give the decompressed size and the payload.
4. If `size_unzipped != 0`, the payload is gzip-inflated through `zip::inflate(&message_body, size_unzipped)`; a failure logs `"Unable to decompress buffer"` and drops the batch.
5. The decompressed payload is a sequence of length-prefixed messages: `u32 length` followed by `length` bytes, read with `BinaryReader`. Each inner message is fed back into `log_net_message`, so it gets its own file and its own sequence number.

`log_console` writes to `GetStdHandle(STD_OUTPUT_HANDLE)`. `open_file`, `close_file`, `log_open_file` and `delete_file` are direct ports of the C++ `CLogger` members; they are part of the public API but nothing in the crate calls them.
**Notes and gotchas:** The `.tmp` → `.bin` rename is what makes the capture directory safe to read while Steam is running: a file with its final name is always complete. `msg_num` is an `AtomicU32` with `Relaxed` ordering — the counter is a naming device, not a synchronisation point, and its value under concurrent capture depends on the order in which the two hooks happen to interleave. `message_name` is called from inside `file_name_base`, i.e. it calls **into the Steam client** (`PchMsgNameFromEMsg`) from the hook; if that lookup was never resolved, the name degrades to `"(null)"` and the file name still parses, because the analyzer only needs the first three components. `multiplex_multi` clamps `header_end` to `data.len()` and `header_length` to `>= 0`; the C++ did not clamp and would happily read past the buffer. `log_net_message` returns early for buffers shorter than two bytes, which the C++ did not do. The `unsafe` blocks are all direct Win32 calls: `CreateDirectoryA`, `DeleteFileA`, `CreateFileA`, `WriteFile`, `MoveFileA`, `SetFilePointer`, `GetStdHandle`, `CloseHandle`, `GetModuleFileNameA`.

#### `crates/steamkit-nethook/src/net.rs` — 232 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/net.cpp` and `Resources/NetHook2/NetHook2/net.h`
**Purpose:** Installs the two network hooks of the Steam client — the outgoing WebSocket frame builder and the incoming packet receiver — and forwards what they carry to the logger.
**How it works:** Two byte patterns are matched inside the Steam client library:

| Target | x86-64 pattern | mask | x86 pattern | mask |
|---|---|---|---|---|
| `CWebSocketConnection::BBuildAndAsyncSendFrame` | `48 8B C4 55 48 8D 68 00 48 81 EC 00 00 00 00 48 89 70 00 49 8B F0 48 89 78 00 4C 89 60` (28 bytes) | `xxxxxxx?xxx????xxx?xxxxxx?xxx` | `55 8B EC 83 EC CC A1 CC CC CC CC 53 8B D9 89 5D CC 83 38 CC 75 CC 33 C0 EB CC 8D 45 CC 50 6A CC 68 CC CC CC CC FF 15 CC CC CC CC 83 C4 CC 83 7B CC 02` (45 bytes) | `xxxxx?x????xxxxx?xx?x?xxx?xx?xx?x????xx????xx?xx?x` |
| `CCMInterface::RecvPkt` | `48 8B C4 55 48 8D A8 CC CC CC CC 48 81 EC CC CC CC CC 48 89 58 08 48 8B DA 48 89 70 CC 48 8B F1` (30 bytes) | `xxxxxxx????xxx????xxxxxxxxxx?xxx` | `55 8B EC 81 EC CC CC CC CC A1 CC CC CC CC 56 8B F1 57` (18 bytes) | `xxxxx????x????xxxx` |

The patterns are **byte-identical to the C++'s** (the C++ spelled them as escaped string literals). The masks use `?` for "any byte"; every other character compares the byte exactly. `Net::new()` creates a `SimpleScan::with_dll(STEAMCLIENT_DLL)`, calls `find_function(BUILD_SIGNATURE, BUILD_MASK)` and `find_function(RECV_SIGNATURE, RECV_MASK)`, stores the two addresses into the statics `BUILD_AND_ASYNC_SEND_FRAME_ORIG` and `RECV_PKT_ORIG` (both `FnSlot = AtomicPtr<c_void>`), logs the addresses, and — for each function it found — creates a `SimpleDetour` over the corresponding slot and calls `attach()`. A failed scan logs `"… func scan failed."`; a failed attach logs the `DetourError`.

The replacement functions are:

- `b_build_and_async_send_frame(web_socket_connection, op_code, data, len) -> bool` — records the frame as `NetDirection::Outgoing` **only** when `op_code == k_eWebSocketOpCode_Binary`; any other opcode is logged as `"Sending websocket frame with opcode N (Name), ignoring"`. It then calls through the slot, which now holds the trampoline, and returns the original's result. A null slot address short-circuits to `false`.
- `recv_pkt(cm_connection, packet)` — reads `m_pubData` and `m_cubData` out of the `CNetPacket` the Steam client passes in, records the payload as `NetDirection::Incoming`, and then calls the original. The packet pointer is null-checked first.

`init_net()` builds the `Net` and stores it in a global `Mutex<Option<Net>>`; `shutdown_net()` clears it, which drops the two `SimpleDetour`s and therefore detaches the hooks and restores the original bytes.
**Notes and gotchas:** The C++ declared both hooks `__fastcall` with an extra `this`-style first argument on 32-bit (`__thiscall` in `net.h`). On x86-64 there is one calling convention, so the Rust port uses `extern "system"` throughout and **the 32-bit build of the C++ project is not reproduced** — the Rust signatures are x86-64-shaped only. The C++ `net.cpp` carried `static_assert`s pinning `offsetof(CNetPacket, m_pubData) == 16` and `offsetof(CNetPacket, m_cubData) == 24` on 64-bit; the Rust `#[repr(C)] CNetPacket` reproduces that layout but has no equivalent static assertion. The C++ `RecvPkt` dereferenced `pPacket` unconditionally; the Rust version null-checks it. The two `FnSlot` statics are the Rust equivalent of the C++'s `BBuildAndAsyncSendFrame_Orig` / `RecvPkt_Orig` globals — a `void **` becomes an `AtomicPtr<c_void>`, which is the same storage with well-defined concurrent reads. `std::slice::from_raw_parts(data, len)` is the one place the module builds a Rust slice over memory owned by the Steam client; the length comes from the Steam client itself and is trusted, exactly as the C++ trusted `m_cubData`.

#### `crates/steamkit-nethook/src/crypto.rs` — 234 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/crypto.cpp` and `Resources/NetHook2/NetHook2/crypto.h`
**Purpose:** Hooks Steam's symmetric encryption so that the **plaintext** of every outbound message is captured, and resolves Steam's own message-number-to-name function so that captured files get readable names.
**How it works:** Two byte patterns are matched in the Steam client library:

| Target | x86-64 pattern | mask | x86 pattern | mask |
|---|---|---|---|---|
| `CCrypto::SymmetricEncryptChosenIV` | `48 83 EC 58 8B 84 24 CC CC CC CC C6 44 24` (14 bytes) | `xxxxxxx????xxx` | `55 8B EC 6A CC FF 75 CC FF 75 CC FF 75 CC FF 75 CC FF 75 CC FF 75 CC FF 75 CC FF 75 CC FF 75` (35 bytes) | `xxxx?xx?xx?xx?xx?xx?xx?xx?xx` |
| `PchMsgNameFromEMsg` | `48 89 5C 24 CC 57 48 83 EC 20 8B D9 E8` (13 bytes) | `xxxx?xxxxxxxx` | `55 8B EC 51 56 E8 00 00 00 00 8B 00 00 00 00 00 8B F0` (18 bytes) | `xxxxxx????x?????xx` |

The encryption hook is installed exactly like the two network hooks: `Crypto::new()` scans, stores the address in the `ENCRYPT_ORIG` slot, creates a `SimpleDetour` and attaches it. The replacement `symmetric_encrypt_chosen_iv(plaintext_data, plaintext_len, iv, iv_len, encrypted_data, encrypted_len, key, key_len) -> bool` records `plaintext_data[..plaintext_len]` as `NetDirection::Outgoing` and then forwards all eight arguments to the original. A null slot short-circuits to `false`.

`PchMsgNameFromEMsg` is **not** hooked — it is only *located*, and stored in `PCH_MSG_NAME_FROM_EMSG`. `message_name(emsg, server_type) -> Option<&'static str>` loads that address, calls it with the message number, and converts the returned `*const c_char` into a `&str`; it returns `None` if the address is null or the returned pointer is null.

`MsgInfo` is a `#[repr(C)]` copy of the C++ `MsgInfo_t` (`eMsg`, `nFlags`, `k_EServerTarget`, `nUnk1`, `pchMsgName`). The C++ used it only for `static_assert`s on its size and field offsets; the port keeps the structure so the layout assertion is still expressed, but the structure is not otherwise used.
**Notes and gotchas:** The C++'s `CCrypto::GetMessage` returned `nullptr` when `PchMsgNameFromEMsg` was null, and the `%s` conversion in `GetFileNameBase` then printed the literal text `(null)`. The Rust chain reproduces this exactly: `message_name` returns `None` and `logger.rs` substitutes `"(null)"`. The `server_type` parameter is **ignored** in both the C++ and the Rust (`_server_type` in the Rust signature) — the C++ passed it to `GetMessage`, which never used it either. The `SymmetricEncryptChosenIVFn` type is `extern "C"` (the C++ used `__cdecl`), while the network hooks are `extern "system"` — the difference mirrors the C++, where `CCrypto::SymmetricEncryptChosenIV` was `__cdecl` and `CNet::BBuildAndAsyncSendFrame` was `__fastcall`. Because the crypto hook records the plaintext, a message that the client sends is captured **twice** over the session: once encrypted-frame shaped by the WebSocket hook and once as plaintext by the crypto hook. The C++ had exactly the same property. The `unsafe` blocks are `std::slice::from_raw_parts` over the plaintext buffer, the `transmute` of the resolved address into a function pointer, and the call through it.

#### `crates/steamkit-nethook/src/csimpledetour.rs` — 909 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/csimpledetour.cpp` and `Resources/NetHook2/NetHook2/csimpledetour.h` (the C++ version is a thin wrapper around Microsoft Detours)
**Purpose:** The function-hooking engine. This module replaces the Detours dependency with a self-contained inline trampoline patcher: it relocates the prologue of the target function into a trampoline, patches the target with a jump to the replacement, and re-points the caller's function pointer at the trampoline so calls still reach the original code.
**How it works:** `SimpleDetour` holds a `&'static FnSlot` (the `AtomicPtr<c_void>` that holds the address of the function to hook), the replacement address, the original address, the trampoline address, the saved original bytes and an `attached` flag. `FnSlot` is the Rust stand-in for the C++'s `void **`; the atomic gives the same storage with well-defined concurrent reads from the hook itself.

`attach()`:

1. Read the target address from the slot; null → `DetourError::NullTarget`.
2. `suspend_other_threads()` — the equivalent of Detours' transaction, which suspends every thread of the process while the code is being patched.
3. `install_hook(original, replacement)`:
   - `build_trampoline(target)` — decode the prologue, relocate it, allocate the trampoline;
   - `build_patch(target, replacement)` — the jump that goes over the prologue;
   - `write_code(target, &patch)` — overwrite the target.
4. Store the trampoline address **into the slot**, so the replacement function's call-through-the-slot reaches the original prologue and then the rest of the original function.

`detach()` restores the saved bytes, frees the trampoline (`VirtualFree(MEM_RELEASE)`), and puts the original address back into the slot. `Drop for SimpleDetour` calls `detach()`, so dropping the `Net`/`Crypto` structs — which is what `shutdown_net()`/`shutdown_crypto()` do — removes the hooks.

**Trampoline construction.** `readable_bytes_at(target)` uses `VirtualQuery` to find how many bytes of the region are readable (capped at `MAX_PROLOGUE = 64`). The prologue is then decoded instruction by instruction with the hand-written `decoder` module until at least `patch_size()` bytes are covered — 14 bytes on x86-64, 5 on x86. Each instruction is given an *emitted* length that may exceed its original length, because short branches must be widened: `jcc rel8` → 6 bytes (`0F 8x rel32`), `jmp rel8` → 5 bytes (`E9 rel32`), `loop/jcxz rel8` → 7 bytes (`EB 05` + `E9 rel32`, the short jump skipping over the long one so the loop semantics survive). Branches that target another instruction *inside* the relocated prologue are re-pointed at the trampoline's copy; everything else keeps its absolute target. RIP-relative memory operands are re-based so they still address the same absolute address. The trampoline is allocated with `VirtualAlloc(MEM_COMMIT|MEM_RESERVE, PAGE_EXECUTE_READWRITE)`, the image is written into it, and a jump back to `target + original_len` is appended (x86-64: `FF 25 00 00 00 00` followed by the 8-byte absolute address; x86: `E9 rel32`). The original prologue bytes are saved verbatim so they can be restored on detach.

**The patch.** `build_patch` writes the same 14-byte absolute jump (`FF 25 00 00 00 00` + 8-byte address) over the target on x86-64, or a 5-byte `E9 rel32` on x86. `write_code` flips the page protection to `PAGE_EXECUTE_READWRITE` with `VirtualProtect`, copies the bytes with `std::ptr::copy_nonoverlapping`, restores the original protection, and calls `FlushInstructionCache`.

**The decoder.** A private `mod decoder` implements an x86/x86-64 length decoder: legacy prefixes and REX, the one-byte opcode map, the two-byte `0F` map (including the three-byte `0F 38` / `0F 3A` maps), ModRM/SIB/displacement parsing, RIP-relative detection on x86-64, and immediate-operand sizing. It returns `None` when it cannot decode, which makes `attach` fail with `DetourError::DecodeFailed` rather than corrupt the target. `RelKind` distinguishes `ShortJump`, `ShortJumpUnconditional`, `ShortLoop` and `Long`.

**Thread suspension.** `SuspendedThreads::suspend()` takes a `CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD)` snapshot, walks it with `Thread32First`/`Thread32Next`, and suspends every thread of the current process except the calling one with `SuspendThread`. The **previous** suspend count is recorded per thread, and `Drop` resumes only those threads whose previous count was zero — a thread that was already suspended by someone else is left alone.
**Notes and gotchas:** The `DetourError` enum enumerates every failure mode: `NullTarget`, `DecodeFailed`, `UnsupportedInstruction`, `AllocationFailed`, `DisplacementOutOfRange`, `ProtectionFailed`, `QueryFailed`. `UnsupportedInstruction` is declared but never produced — the decoder returns `None` (→ `DecodeFailed`) instead. The decoder is **not a complete x86 decoder**: it does not model VEX/EVEX prefixes (`0xC4`/`0xC5` are treated as plain ModRM-bearing opcodes), it does not model the extra immediate byte of the `0F 3A` map beyond the `0x3A` group, and it treats `0xA0..0xA3` (`mov al, [moffs]`) as an 8-byte absolute immediate on x86-64. A prologue containing such an instruction would be mis-decoded — the scanner would then patch at the wrong offset. In practice the hooked prologues are ordinary MSVC frames (`mov r11, rsp` / `push rbp` / `sub rsp, …`), which the decoder handles. `SimpleDetour` carries `unsafe impl Send` and `unsafe impl Sync` with a justification: the hook is installed and removed during `DLL_PROCESS_ATTACH`/`DLL_PROCESS_DETACH`, before another thread can reach the hooked function. The `unsafe` blocks are: `install_hook`, `build_trampoline`, `build_patch`, `write_code`, `readable_bytes_at`, `read_bytes` and `emit_instruction` (all `unsafe fn`), the two `copy_nonoverlapping` calls, the `std::ptr::read_unaligned` PE reads in `sigscan.rs` (same pattern), and the thread-snapshot block. **This module is the second least-proven part of the port**: the trampoline has never been exercised against a real Steam client.

#### `crates/steamkit-nethook/src/csimplescan.rs` — 92 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/csimplescan.cpp` and `Resources/NetHook2/NetHook2/csimplescan.h`
**Purpose:** Binds a byte-pattern scanner to a loaded module and finds functions inside it by signature.
**How it works:** `SimpleScan` holds `interface_set: bool`, `interface: *mut c_void` and an embedded `SigScan`. `set_dll(filename)` calls `sys_get_factory(filename)` — which resolves `GetModuleHandleA(filename)` and then `GetProcAddress(module, "CreateInterface")` — stores that function pointer through `sigscan::set_dllfunc`, and then asks `SigScan::get_dll_mem_info()` to work out the module's base address and image size. The scan is only usable if that succeeded *and* the factory pointer is non-null. `find_function(sig, mask)` then calls `SigScan::init(sig, mask)` and returns `SigScan::sig_addr` when `init` returned `0` and `is_set` is true.

The choice of `CreateInterface` as the anchor function is the C++'s: Valve's modules export it, so the address of that function identifies the module unambiguously, and `VirtualQuery` on it yields the allocation base of the PE image.
**Notes and gotchas:** The C++ `Sys_GetProcAddress` had `assert(hModule != nullptr)` and would abort the process if the module was missing; the Rust returns `None` and the scan silently fails to bind, which then makes `find_function` return `None` and the caller log `"… func scan failed."`. `SimpleScan::with_dll(STEAMCLIENT_DLL)` is the only way it is used in the crate. The C++ `CSimpleScan::FindFunction` passed `strlen(mask)` as the pattern length to `CSigScan::Init`; the Rust passes the whole mask slice and `SigScan::init` derives the same length from `mask.len()` — same behaviour, but the length is now *derived* rather than passed separately, so a caller that supplies a mask shorter than the signature silently compares only the leading bytes (see `sigscan.rs`).

#### `crates/steamkit-nethook/src/sigscan.rs` — 198 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/sigscan.cpp` and `Resources/NetHook2/NetHook2/sigscan.h`
**Purpose:** Finds a byte pattern inside a loaded PE module, and works out the base address and the size of that module's image.
**How it works:** Three globals hold the scan state: `SIGSCAN_DLLFUNC` (the anchor function address), `BASE_ADDR` and `BASE_LEN` (the image base and size). `set_dllfunc(func)` stores the anchor.

`get_dll_mem_info()`:

1. `VirtualQuery` on the anchor function address → `MEMORY_BASIC_INFORMATION` → `AllocationBase` is the base of the PE image.
2. Read the DOS header's `e_lfanew` at offset `0x3C` (with `read_unaligned`), bail out if it is zero or greater than `0x1000`.
3. At `base + e_lfanew`, read the NT signature; it must equal `0x0000_4550` (`PE\0\0`).
4. Read `SizeOfImage` from the optional header at `nt + 0x50` (`SIZE_OF_IMAGE_OFFSET`).
5. Store base and length in `BASE_ADDR` / `BASE_LEN`.

`init(sig, mask)` returns `0` on success, `1` when the pattern was not found, and `2` when the module information is unavailable — exactly the C++ `CSigScan::Init` convention. It sets `sig_len = mask.len()`, copies that many bytes of the signature into `sig_str`, and copies the mask into `sig_mask`. `find_signature` then slides a window of `sig_len` bytes over the whole image (`std::slice::from_raw_parts(base, len)`) and accepts a window when every masked-in byte matches: the mask character `?` is a wildcard, **any other character compares the byte exactly**, including `0`.
**Notes and gotchas:** The C++ used the **mask length as the signature length** (`CSigScan::Init(sig, mask, strlen(mask))`), and the Rust reproduces that deliberately: `sig_str` is truncated to `mask.len()` and then padded with zeroes if the signature was shorter. A mask longer than the signature therefore compares the bytes *past* the end of the signature — which in C++ were the NUL terminator of the string literal, so the comparison still worked; in Rust the pattern is a `&[u8]` slice and the same truncation-and-pad behaviour is reproduced explicitly. In practice every mask in this crate has exactly the same length as its signature. The scan covers the **whole image**, including data sections — the C++ did the same, so a pattern that also appears in a data blob would produce a false positive; the hooked patterns are prologues and are unlikely to collide. `read_unaligned` is required because `e_lfanew` and the NT header fields are not guaranteed to be aligned. The `unsafe` blocks are the `VirtualQuery` call, the `assume_init` of the query result, and the two `read_unaligned` calls that parse the PE headers.

#### `crates/steamkit-nethook/src/sedebug.rs` — 120 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/sedebug.cpp` and `Resources/NetHook2/NetHook2/sedebug.h`
**Purpose:** Acquires the `SeDebugPrivilege` so that the injector can open the protected Steam process and read its module list.
**How it works:** `se_debug_acquire()` opens the **current thread's** token with `OpenThreadToken(GetCurrentThread(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, …)`. If that fails with `ERROR_NO_TOKEN` (no thread token — the usual case on a primary thread), it calls `ImpersonateSelf(SecurityImpersonation)` to materialise one and retries. It then calls `set_privilege(token, b"SeDebugPrivilege\0", true)`; on failure the token is closed and `None` is returned. On success the token handle is handed to the caller, which must close it.

`set_privilege(token, privilege, enable)` does the standard two-pass `AdjustTokenPrivileges` dance: `LookupPrivilegeValueA` resolves the name to a `LUID`; the first `AdjustTokenPrivileges` call reads the current state into `previous_state`; the `SE_PRIVILEGE_ENABLED` bit is then set or cleared in `previous_state`; the second call applies it. `GetLastError()` is checked after each call — `AdjustTokenPrivileges` reports "the privilege was not held" through `GetLastError`, not through its return value.
**Notes and gotchas:** `LookupPrivilegeValueA` is called with a null system name, which resolves the privilege on the local system. The two-pass structure matters: `AdjustTokenPrivileges` *never* fails because of a privilege it could not assign — it succeeds and sets `ERROR_NOT_ALL_ASSIGNED` — so without the `GetLastError` check a caller would believe a privilege was granted when it was not. The `ImpersonateSelf` path is the C++'s as well. The caller in `injector.rs` closes the handle through `close_debug_token`. Both functions are `unsafe fn`: they operate on a raw token handle.

#### `crates/steamkit-nethook/src/binaryreader.rs` — 69 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/binaryreader.cpp` and `Resources/NetHook2/NetHook2/binaryreader.h`
**Purpose:** A forward-only reader over a byte buffer, used to walk the length-prefixed messages inside a multiplexed batch.
**How it works:** `BinaryReader<'a>` borrows a `&'a [u8]` and keeps a `u32` position. `size_left()` returns the remaining byte count (saturating). `read_u32()` reads a little-endian `u32` and always advances the position by four. `read_bytes(len)` returns a slice **into the underlying buffer** — it does not copy — and advances the position by `len`.
**Notes and gotchas:** The C++ `CBinaryReader` handed out raw pointers into the buffer it was constructed with; the Rust port borrows instead, so a read can never outlive the buffer — the lifetime parameter enforces it. Two deliberate differences from the C++: `read_u32` **clamps** (returns `0` when fewer than four bytes remain) where the C++ read out of bounds, and `read_bytes` **clamps** the returned slice to what is actually available where the C++ advanced the position unconditionally and returned a pointer past the end. Both differences turn a potential out-of-bounds read of Steam-supplied data into a short read. The reader is used in exactly one place: the payload loop in `logger.rs::multiplex_multi`.

#### `crates/steamkit-nethook/src/nh2_string.rs` — 29 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/string.cpp` and `Resources/NetHook2/NetHook2/nh2_string.h`
**Purpose:** Two case-insensitive byte-string comparisons used by the injector to match module and process names.
**How it works:** `string_case_insensitive_ends_with(haystack, needle)` returns `false` when the haystack is shorter than the needle, then compares the tail byte by byte with `to_ascii_lowercase` on both sides. `string_case_insensitive_eq(left, right)` compares two whole strings the same way.
**Notes and gotchas:** The C++ helper worked on `const char *` buffers coming from the ANSI flavour of the Win32 API, so the Rust port keeps a byte-oriented interface and folds **ASCII case only**. That matches `_stricmp` for every character that can appear in a Windows path. The functions are used by `injector.rs` (`process_has_module_loaded`, `find_process_by_name`) and by `nethook.rs` (`is_run_dll32`).

#### `crates/steamkit-nethook/src/steamclient.rs` — 8 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steamclient.h`
**Purpose:** Names the Steam client library that NetHook2 hooks into.
**How it works:** A single `pub const STEAMCLIENT_DLL: &str` that is `"steamclient64.dll"` on x86-64 and `"steamclient.dll"` otherwise, selected at compile time with `cfg!(target_arch = "x86_64")`. It is used in three places: `nethook.rs` (`LoadLibraryA` on attach), `net.rs` and `crypto.rs` (the module the signature scanner binds to).
**Notes and gotchas:** `injector.rs` has its own copy of the same choice as `STEAMCLIENT_DLL_BYTES` (a `&[u8]`, because it is compared against module *paths* rather than passed to `LoadLibraryA`). The two constants must stay in sync; they are selected by the same `cfg!` predicate but are not derived from one another.

#### `crates/steamkit-nethook/src/version.rs` — 25 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/version.h` (and the `version.cpp` that `GenerateVersionInfo.ps1` generated)
**Purpose:** Exposes the four build-time values that `build.rs` stamped into the binary, so that `nethook.rs` can print them at start-up.
**How it works:** `BUILD_DATE`, `BUILT_FROM_COMMIT_SHA` and `BUILT_FROM_COMMIT_DATE` are `const &str` read from `option_env!("NETHOOK_BUILD_DATE")`, `option_env!("NETHOOK_COMMIT_SHA")` and `option_env!("NETHOOK_COMMIT_DATE")`; `built_from_dirty()` returns `true` when `option_env!("NETHOOK_DIRTY") == Some("1")`. `option_env!` (rather than `env!`) means a missing value degrades gracefully: the date becomes `"unknown"` and the SHA and the commit date become the empty string — which is what `nethook.rs::print_version_info` tests for to decide whether to print `"No further build information available."`.
**Notes and gotchas:** The C++ declared these as mutable globals in a generated `version.cpp`; here they are compile-time constants, so they cannot be changed at run time — which is fine, because nothing changed them. The C++ additionally stamped a Windows `VERSIONINFO` resource into the DLL; **that is not reproduced**, so the built DLL has no version resource and Explorer shows no product name for it.

#### `crates/steamkit-nethook/src/zip.rs` — 33 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/zip.cpp` and `Resources/NetHook2/NetHook2/zip.h`
**Purpose:** Decompresses the gzip payload of a multiplexed message batch.
**How it works:** `inflate(compressed: &[u8], decompressed_len: u32) -> Option<Vec<u8>>` allocates a buffer of exactly `decompressed_len` bytes and feeds it to a `flate2::read::GzDecoder` until the buffer is full. Then it performs **one more read** of a single byte: if that read returns `Ok(0)` the gzip stream ended exactly at `decompressed_len`, and the buffer is returned; anything else means the stream is longer, truncated or corrupt, and `None` is returned.
**How it works (continued):** The C++ used zlib's `inflateInit2(&zstrm, 16)`. In zlib, a window size of 16 means **gzip decoding only** — the zlib container is rejected with `Z_DATA_ERROR`. That matches `SteamKit2`'s own `CMsgMulti` handling, which wraps the payload in a `GZipStream`. The Rust port therefore uses `flate2::read::GzDecoder` and not `ZlibDecoder`.
**Notes and gotchas:** The "one more read" probe is what makes the Rust behaviour identical to the C++'s, which only accepted `Z_STREAM_END`. A stream that decompresses to *fewer* bytes than `decompressed_len` leaves the tail of the buffer zero-filled and the probe read returns `Ok(0)`, so it is accepted — the C++ had the same tolerance, since `inflate` returning `Z_STREAM_END` with a shorter output was also accepted. A `None` result makes `logger.rs` log `"Unable to decompress buffer"` and drop the whole batch, so a corrupt gzip loses every message inside it, not just one. `flate2` is the only dependency in the crate that is not a Win32 or protobuf library.

#### `crates/steamkit-nethook/src/steam/mod.rs` — 7 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/` (the directory of hand-written Steam protocol headers)
**Purpose:** Declares the submodules that mirror Valve's hand-written Steam headers.
**How it works:** Five `pub mod` lines: `clientmsgs`, `csteamid`, `emsg`, `net`, `steamtypes`.
**Notes and gotchas:** **`udppkt` is missing from this list.** `src/steam/udppkt.rs` exists and is a faithful port of `NetHook2/steam/udppkt.h`, but because `mod.rs` does not declare it, the file is **never compiled** — it is an orphan. Adding `pub mod udppkt;` would compile it, and it would then produce `dead_code` warnings for most of its items (nothing in the crate reads `UDPPktHdr_t` or `MsgHdr_t`; the analyzer has its own header parsing in `crates/steamkit-tools/src/nethook/`). The C++ project had the same file, and it was equally unused there — the port reproduced the file but not the (unused) declaration.

#### `crates/steamkit-nethook/src/steam/emsg.rs` — 38 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/emsg.h`
**Purpose:** Provides the Steam message type as it appears on the wire.
**How it works:** `EMsg` is a `#[repr(transparent)]` newtype over `u32` with `from_raw`, `raw`, and `From<u16>` / `From<u32>` conversions. The only named member is `MULTI: EMsg = EMsg(1)` — Valve's `k_EMsgMulti`, the sentinel that marks a multiplexed batch and is the one value the logger actually compares against.
**Notes and gotchas:** This is **not** Valve's full `EMsg` enumeration. The C++ `emsg.h` carried roughly 1400 members; the Rust module keeps only the wire value, because NetHook2 itself only ever needs the *numeric* identity of a message type plus the `k_EMsgMulti` sentinel. The human-readable name is resolved at run time by Steam's own `PchMsgNameFromEMsg` (see `crypto.rs`), not by this header. The full enumeration already exists in the workspace as `steamkit::base::generated::steam_language::EMsg`, generated by `steamkit-lang-gen` from Valve's `Resources/SteamLanguage/emsg.steamd` — duplicating 1400 members into the DLL crate would have been pure copy/paste with no behavioural gain. The crate depends on `steamkit-protos`, not on `steamkit`, so it cannot use that generated enumeration; the two are deliberately separate.

#### `crates/steamkit-nethook/src/steam/net.rs` — 77 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/net.h`
**Purpose:** Declares the WebSocket opcode enumeration and the `CNetPacket` structure that the `RecvPkt` hook reads out of.
**How it works:** `HCONNECTION` is a `u32`. `EWebSocketOpCode` is a `#[repr(i32)]` enum with `Continuation = 0`, `Text = 1`, `Binary = 2`, `Close = 8`, `Ping = 9`, `Pong = 0x0A`. `e_web_socket_op_code_to_name` indexes a 16-entry name table and returns `"<Invalid>"` for anything outside it (the reserved slots are named `"<Reserved>"`). `CNetPacket` is `#[repr(C)]` with `m_hConnection: HCONNECTION`, `m_unUnk1: u32`, `m_unUnk2: u32`, `m_pubData: *mut u8`, `m_cubData: u32`, `m_cRef: i32`, `m_pubNetworkBuffer: *mut u8`.
**Notes and gotchas:** The layout is load-bearing: `net.rs` reads `m_pubData` and `m_cubData` straight out of the object the Steam client passes to the hooked function. On x86-64 the `#[repr(C)]` layout puts `m_pubData` at offset 16 and `m_cubData` at offset 24 — exactly the offsets the C++ pinned with `static_assert(offsetof(CNetPacket, m_pubData) == 16, …)` and `static_assert(offsetof(CNetPacket, m_cubData) == 24, …)` in `net.cpp`. The Rust port does not carry those static assertions; the `#[repr(C)]` attribute plus the field types reproduce the layout, and a field reordering would silently break the capture. On 32-bit the C++ asserted offsets 12 and 16, which the same `#[repr(C)]` layout also produces. `EWebSocketOpCode` has no `#[non_exhaustive]`, and the `From<i32>` conversion does not exist, so a value outside the listed variants cannot be constructed in safe Rust — but the Steam client can pass one, and the hook only compares against `Binary` and prints the raw integer for anything else.

#### `crates/steamkit-nethook/src/steam/clientmsgs.rs` — 110 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/clientmsgs.h`
**Purpose:** Declares the handful of client message structures the original NetHook2 headers carried.
**How it works:** Seven `#[repr(C, packed)]` structures: `MsgChannelEncryptRequest_t` (`m_unProtocolVer`, `m_EUniverse`), `MsgChannelEncryptResponse_t` (`m_unProtocolVer`, `m_cubEncryptedKey`), `MsgChannelEncryptResult_t` (`m_EResult`), `MsgMulti_t` (`m_cubUnzipped`), `MsgClientAnonLogOn_t` (`m_unProtocolVer`, `m_unIPPrivateObfuscated`, `m_unIPPublic`), `MsgClientLogOnResponse_t` (`m_EResult`, heartbeat rates, `m_ulClientSuppliedSteamId: CSteamID`, `m_unIPPublic`, `m_RTime32ServerRealTime: RTime32`) and `MsgClientLogOnWithCredentials_t` (protocol version, IPs, Steam id, ticket length, a 64-byte account name, a 20-byte password, `m_qosLevel`), plus `MsgClientRegisterAuthTicketWithCM_t`.
**Notes and gotchas:** These structures are **declarations only** — nothing in the crate instantiates or reads them. They are the Rust counterparts of the C++ headers and exist so that the port of `steam/` is complete. `#[repr(C, packed)]` matches the C++ header's `#pragma pack` behaviour: `MsgClientLogOnWithCredentials_t` in particular has a `u64` Steam id inside a packed struct, which means the compiler inserts no padding and the field is potentially unaligned. `MsgClientLogOnWithCredentials_t` does **not** derive `Default` or `PartialEq` (unlike its siblings) because its `[u8; 64]` / `[u8; 20]` arrays make the derived implementations unwieldy; that asymmetry is inherited from the C++ header's own asymmetry.

#### `crates/steamkit-nethook/src/steam/csteamid.rs` — 279 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/csteamid.h`
**Purpose:** Valve's 64-bit Steam id, with the bit-level accessors that the C++ exposed through a union.
**How it works:** `EUniverse` (`k_EUniverseInvalid`=0 … `k_EUniverseRC`=5, `K_EUNIVERSE_MAX`=6) and `EAccountType` (`k_EAccountTypeInvalid`=0 … `k_EAccountTypeAnonUser`=10, `K_EACCOUNT_TYPE_MAX`=11) are `#[repr(u8)]` enums. `CSteamID` wraps a single `m_unAll64Bits: u64` and derives every field on demand:

| Field | Bits | Accessors |
|---|---|---|
| account id | 0–31 | `get_account_id` / `set_account_id` |
| account instance | 32–51 (20 bits) | `get_account_instance` / `set_account_instance` |
| account type | 52–55 (4 bits) | `get_account_type` / `set_account_type` |
| universe | 56–63 (8 bits) | `get_universe` / `set_universe` |

Construction helpers: `new`, `from_uint64`, `from_parts`, `set` (which defaults the instance to 1, or 0 for a clan account), `instanced_set`, `full_set` (low 52 bits + universe + type) and `set_from_uint64`. Predicates: `is_individual_account`, `is_game_server_account` (game server *or* anonymous game server), `is_content_server_account`, `is_clan_account`, `is_chat_account`, `is_anon_account` (anonymous user *or* anonymous game server), `is_anon_user_account`, `is_lobby` (a chat account whose instance carries `K_E_CHAT_INSTANCE_FLAG_LOBBY`), `is_blank_anon_account` and `is_valid` (the port of `CSteamID::IsValid`: the account type must be in range, the universe must be in range, an individual account must have a non-zero id and instance 1, a clan account must have a non-zero id and instance 0). The chat-instance flag constants `K_E_CHAT_INSTANCE_FLAG_CLAN`, `_LOBBY` and `_MMS_LOBBY` are derived from `K_UN_STEAM_ACCOUNT_INSTANCE_MASK` the way Valve derives them.
**Notes and gotchas:** The C++ exposed the bit fields through a **union**, which meant reading a field that had not been written was undefined behaviour in practice. The Rust port keeps the raw 64-bit value and derives the fields on demand — equivalent, and it removes the uninitialised-member problem the C++ union had. `K_STEAM_ID_NIL` is defined as `CSteamID::new()`, i.e. all zeroes; the C++'s `k_SteamIDNil` was `0` as well. Nothing in the crate actually constructs or compares a `CSteamID` — the type exists so that `clientmsgs.rs` and `steamtypes.rs` can name it, and so that the port of `steam/` is complete.

#### `crates/steamkit-nethook/src/steam/steamtypes.rs` — 361 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/steamtypes.h`
**Purpose:** Valve's fixed-width type aliases, the `EResult` and `EServerType` enumerations, and a few small structures from `steamtypes.h`.
**How it works:** The type aliases are `SteamUnsigned64`, `Gid` (`u64`), `JobId`, `AppId`, `PackageId`, `DepotId`, `SteamApiCall`, `HSteamPipe`, `HSteamUser`, `HSteamCall`, `HAuthTicket`, `RTime32` and `SteamInstanceID` / `SteamLocalUserID`. The constants include `K_GID_NIL` / `K_TXN_ID_NIL` (`0xFFFF_FFFF_FFFF_FFFF`), `K_TXN_ID_UNKNOWN` (`0`), the `K_U_*_INVALID` sentinels, `K_RTIME32_NIL` / `_MIN_VALID` / `_INFINITE`, `K_N_MAGIC` (`0x3130_5356`, "VS01"), `K_N_MAGIC_OLD1` ("VSTM"), `K_CCH_TRUNCATED_PASSWORD` (20), `K_CCH_ACCOUNT_NAME` (64), `K_N_CHALLENGE_MASK` and `K_N_OBFUSCATION_MASK`.

`EResult` is a `#[repr(i32)]` enum with the 55 values the C++ header carried (`k_EResultOK` = 1 … `k_EResultRemoteCallFailed` = 55). `EServerType` is a `#[repr(i32)]` enum with `k_EServerTypeInvalid` = -1 through `k_EServerTypeMax` = 50, including the obsolete entries (`HubOBSOLETE`, `ASBOBSOLETE`, `BootstrapOBSOLETE`, `P2PRelayOBSOLETE`). `CreateInterfaceFn` is `unsafe extern "C" fn(*const i8, *mut i32) -> *mut c_void` — Valve's interface factory signature. Also `TSteamSplitLocalUserID`, `TSteamGlobalUserID` and `CallbackMsg_t`.
**Notes and gotchas:** `EServerType` is referenced by `crypto.rs`'s `MsgInfo`, but the *value* is never used — `message_name` ignores the server type entirely, exactly as the C++ `CCrypto::GetMessage` did. `EResult` and `EServerType` are **not** the workspace's canonical enumerations: those live in `steamkit::base::generated::steam_language` (generated by `steamkit-lang-gen` from Valve's `.steamd` files). The DLL crate does not depend on `steamkit`, so it carries its own trimmed copies; they are declarations only and nothing in the crate matches on them. `CallbackMsg_t` holds a raw `*mut u8` payload pointer — the only raw pointer in this module, and it is never dereferenced anywhere in the crate.

#### `crates/steamkit-nethook/src/steam/udppkt.rs` — 126 lines
**C# counterpart:** `Resources/NetHook2/NetHook2/steam/udppkt.h`
**Purpose:** Declares Valve's UDP packet header and the two Steam 3 message header layouts.
**How it works:** The `K_U_NET_FLAG_*` constants (`NO_IOCP`=1, `FIND_AVAIL_PORT`=2, `USE_AUTHENTICATION`=4, `USE_ENCRYPTION`=8, `RAW_STREAM`=16, `RAW_STREAM_SEND`=32, `UNBOUND_SOCKET`=64, `RAW_IO_RECV`=128, and the derived `K_U_NET_FLAGS_KEY_CALLBACK_REQUIRED`). `EUDPPktType` is a `#[repr(u8)]` enum (`ChallengeReq`=1 … `Max`=8). `UDPPktHdr_t` is a `#[repr(C, packed)]` structure with the magic (`"VS01"` / `"VT01"`), the packet size, the type, the flags, the source and destination connection ids and the sequence/acknowledgement counters. `MsgHdr_t` and `ExtendedClientMsgHdr_t` are the `#[repr(C, packed)]` Steam 3 message headers, the extended one carrying the canary, the sender's `CSteamID` and the session id. `emsg_from_raw(value: u16)` is a convenience constructor.
**Notes and gotchas:** **This file is not compiled.** `src/steam/mod.rs` declares `clientmsgs`, `csteamid`, `emsg`, `net` and `steamtypes` — but **not** `udppkt` — so the module is an orphan file that the compiler never sees. Nothing in the crate uses `UDPPktHdr_t` or `MsgHdr_t`: the analyzer in `crates/steamkit-tools/src/nethook/` parses the message headers itself from the protobuf definitions, and the DLL only ever looks at the first two bytes of a payload (the `EMsg`). Adding `pub mod udppkt;` to `mod.rs` would compile it cleanly and produce `dead_code` warnings for every item.

#### 8.7 The `unsafe` discipline, file by file

`steamkit-nethook` is the only crate in the workspace that is not
`#![forbid(unsafe_code)]`. The discipline it follows is: **every `unsafe` block
carries a `SAFETY:` comment naming the invariant it relies on**, and no `unsafe`
block is wider than the single Win32 call it wraps.

| File | Where the `unsafe` is | Invariant relied on |
|---|---|---|
| `src/nethook.rs` | `GetModuleFileNameA`, `AllocConsole`, `LoadLibraryA`, `FreeConsole` | The buffer is `MAX_PATH` bytes and the API NUL-terminates it; the DLL name is a `&'static str`, so `as_ptr()` is already NUL-terminated; the console is only freed if this DLL allocated it (`OWNS_CONSOLE`). |
| `src/injector.rs` | `MessageBoxA`, `se_debug_acquire`, `CloseHandle`, `CStr::from_ptr`, `GetModuleFileNameA`/`W`, `EnumProcesses`, `OpenProcess`, `GetModuleFileNameExA`, `IsWow64Process2`, `EnumProcessModules`, `GetModuleHandleA`, `GetProcAddress`, `VirtualAllocEx`, `WriteProcessMemory`, `CreateRemoteThread`, `WaitForSingleObject`, `VirtualFreeEx` | Every pointer is either a NUL-terminated literal, a `MAX_PATH` buffer sized for the API, or a handle the same function opened and closes on every path. `std::mem::transmute` of `GetProcAddress` results into `LPTHREAD_START_ROUTINE` is the one reinterpretation, and it is only applied to a real kernel32 export. |
| `src/logger.rs` | `CreateDirectoryA`, `DeleteFileA`, `CreateFileA`, `WriteFile`, `MoveFileA`, `SetFilePointer`, `GetStdHandle`, `CloseHandle`, `GetModuleFileNameA` | All paths are built through `nul_terminated()`, which appends the NUL the ANSI API requires; every handle is closed on every path; the `GetStdHandle` fallback is an explicitly null handle, and the subsequent `WriteFile` failure is ignored. |
| `src/net.rs` | `std::slice::from_raw_parts` over the frame buffer and over `m_pubData`/`m_cubData`; `std::mem::transmute` of the slot address into the original function's type | The length is supplied by the Steam client itself (`cubData` / `m_cubData`) and is trusted exactly as the C++ trusted it; the slot address is either the original function or the trampoline the detour installed, and is null-checked before the transmute. |
| `src/crypto.rs` | `std::slice::from_raw_parts` over the plaintext buffer; `transmute` of the resolved `PchMsgNameFromEMsg` address; `CStr::from_ptr` over the returned name | The name pointer is owned by the Steam client and is NUL-terminated; the pointer is null-checked before `CStr::from_ptr`. |
| `src/csimpledetour.rs` | `install_hook`, `build_trampoline`, `build_patch`, `write_code`, `readable_bytes_at`, `read_bytes`, `emit_instruction` (all `unsafe fn`); two `copy_nonoverlapping`; the thread-snapshot block | The target must be mapped executable code; `readable_bytes_at` bounds the read by the `VirtualQuery` region; the trampoline is `VirtualAlloc`ed with exactly `image_size` writable bytes before the copy; `VirtualProtect` makes the target writable before it is written and restores the original protection afterwards; `FlushInstructionCache` is called after every write. |
| `src/csimplescan.rs` | `GetModuleHandleA`, `GetProcAddress` | Both names are NUL-terminated `CString`s; a missing module or export yields `None`, not a null dereference. |
| `src/sigscan.rs` | `VirtualQuery`, `assume_init`, two `read_unaligned` PE-header reads | The query succeeded before `assume_init`; the allocation base is the start of a mapped PE image, `e_lfanew` is bounds-checked against `0x1000`, and the NT signature is verified before `SizeOfImage` is trusted. |
| `src/sedebug.rs` | The whole module is `unsafe fn` (`set_privilege`, `se_debug_acquire`) | The token handle is one this function opened; the caller closes it. |

The two `unsafe impl` blocks in the crate are in `csimpledetour.rs`:
`unsafe impl Send for SimpleDetour` and `unsafe impl Sync for SimpleDetour`, with
the justification that the hook is installed and removed during
`DLL_PROCESS_ATTACH` / `DLL_PROCESS_DETACH`, before another thread can reach the
hooked function.

#### 8.8 End-to-end data flow

```
                      rundll32.exe "NetHook2.dll,Inject"          (src/injector.rs)
                                     │
                                     ├─ find steam.exe ──────────────┐
                                     ├─ acquire SeDebugPrivilege ────┤ (src/sedebug.rs)
                                     ├─ VirtualAllocEx + WriteProcessMemory
                                     └─ CreateRemoteThread(LoadLibraryW)
                                                                          │
  ┌───────────────────────────────────────────────────────────────────────▼──────┐
  │ Steam.exe process                                                            │
  │                                                                              │
  │  DllMain (src/nethook.rs)                                                   │
  │    ├─ is_run_dll32() → false (we are in Steam, not rundll32)                │
  │    ├─ AllocConsole, LoadLibraryA("steamclient64.dll")                      │
  │    ├─ init_logger()  → <Steam dir>\nethook\<epoch>\  (src/logger.rs)      │
  │    ├─ init_crypto()  → SimpleScan → SimpleDetour::attach (src/crypto.rs)   │
  │    └─ init_net()     → SimpleScan → SimpleDetour::attach (src/net.rs)      │
  │                                                                              │
  │  CWebSocketConnection::BBuildAndAsyncSendFrame ──► hook ──► log_net_message │
  │  CCMInterface::RecvPkt                      ──► hook ──► log_net_message     │
  │  CCrypto::SymmetricEncryptChosenIV          ──► hook ──► log_net_message     │
  │                                                                              │
  │  log_net_message                                                             │
  │    ├─ EMsg == 1 (Multi) → multiplex_multi → inflate → per-message recurse  │
  │    └─ else              → log_session_data → "<seq>_<dir>_<emsg>_<name>"   │
  │                           write "<name>.tmp" → MoveFileA → "<name>.bin"      │
  └──────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
                    <Steam dir>\nethook\<epoch>\001_out_700_ClientLogon.bin
                                     │
                                     ▼
              steamkit-nethook-analyzer  (crates/steamkit-tools/src/nethook/)
                parse_capture_file_name → (?<num>\d+)_(?<direction>in|out)_(?<emsg>\d+)
                decode the protobuf body → tree / list view
```

#### 8.9 Summary of deviations from the C++ original

| # | Deviation | Where | Why |
|---|---|---|---|
| 1 | Detours replaced by an in-crate trampoline patcher | `src/csimpledetour.rs` | Removes the vcpkg/MSVC external dependency; the C++ pulled Detours in through vcpkg. |
| 2 | Protobuf from `steamkit-protos` instead of the checked-in `steammessages_base.pb.{h,cc}` | `Cargo.toml`, `src/lib.rs` | The same messages are already generated by `prost`; the 19.5k lines of generated C++ are not ported. |
| 3 | `LoadLibraryW` + UTF-16 path instead of `LoadLibraryA` + ANSI path | `src/injector.rs` | Install paths the ANSI code page cannot represent now work. |
| 4 | The ejector is hand-assembled machine code instead of a copy of a C++ function | `src/injector.rs` (`EJECT_CODE`) | A Rust function is not position independent and references the Rust runtime, so it cannot be copied into the target process. |
| 5 | The C++'s un-terminated `LoadLibraryA` argument is fixed | `src/injector.rs` | The C++ allocated and wrote `strlen(path)` bytes with no NUL terminator. |
| 6 | `__fastcall` / `__thiscall` hooks replaced with the system ABI | `src/net.rs` | `__fastcall` has no 64-bit definition, so the 32-bit build of the C++ project is not reproduced. |
| 7 | Bounds clamping in `BinaryReader` | `src/binaryreader.rs` | The C++ read out of bounds on truncated Steam-supplied data. |
| 8 | `EMsg` is a numeric newtype, not the full enumeration | `src/steam/emsg.rs` | The names come from Steam at run time; the full enumeration lives in `steamkit::base::generated::steam_language::EMsg`. |
| 9 | `udppkt` is present but not declared in `mod.rs` | `src/steam/mod.rs`, `src/steam/udppkt.rs` | The file is a port of the C++ header but is not compiled; nothing in the crate uses it. |
| 10 | No Windows `VERSIONINFO` resource | `src/version.rs` | Resource-script concern; the four version values are still stamped and printed. |




---

## 9. The generated and vendored code

Not everything in this repository is hand-written, and the difference matters when you go
looking for something.

### 9.1 `vendor/Protobufs/` — Valve's protobuf definitions

A git checkout of Valve's protobuf repository, pinned at commit **`bdb3957d`** (2026-11-04).
538 `.proto` files across `steam/`, `webui/`, `csgo/`, `dota2/`, `tf2/`, `deadlock/`,
`underlords/` and `artifact/`.

This is the upstream source of truth. It is **not** what the build compiles — the build uses
the staged subset below — but it is what you re-stage from when Valve updates.

### 9.2 `crates/steamkit-protos/proto/` — the staged subset

155 `.proto` files, produced by `tools/stage_protos.py` from the checkout above. This is
what `prost-build` actually compiles. The staging step applies the two patches described in
[§3.5](#35-the-protobuf-strategy).

Run it with:

```powershell
python tools\stage_protos.py
```

### 9.3 `crates/steamkit/src/base/generated/steam_language.rs` — the 91 Valve enums

17,863 lines, generated by `steamkit-lang-gen` from the seven `.steamd` files in
`crates/steamkit-tools/lang/`. **Regenerate it, do not edit it.** The generator is the Rust
port of the C# `Resources/SteamLanguageParser`.

The `.steamd` format is a Valve-specific DSL for declaring enum constants:

```
enum EMsg
{
    Invalid = 0;
    Multi = 1;
    ProtobufWrapped = 2;
    ...
}
```

### 9.4 The protobuf code itself is not on disk

`prost` writes the generated Rust into `$OUT_DIR` (inside `target/`). The crate's
`src/lib.rs` pulls it in with `include!`:

```rust
include!(concat!(env!("OUT_DIR"), "/steam/steam.rs"));
```

So the 72.6 MB of generated protobuf code exists only during a build. `crates/steamkit-protos/src/`
is a single 1.3 KB file. That is why the checked-in source tree is 11 MB while the *compiled*
surface is much larger.

### 9.5 `crates/steamkit-tools/lang/*.steamd`

The seven Valve enum definition files, 114 KB total:

| File | Contents |
|---|---|
| `emsg.steamd` | `EMsg` — the message numbers |
| `enums.steamd` | the bulk of the shared enums |
| `eresult.steamd` | `EResult` — the result codes |
| `gamecoordinator.steamd` | the GC-specific enums |
| `header.steamd` | the message header structs |
| `netheader.steamd` | the network header structs |
| `steammsg.steamd` | the message-struct definitions |

### 9.6 What is *not* generated

Everything else — all 137 library files, the four tools, the 14 samples, the 33 test files
and the NetHook DLL — is hand-written or hand-ported Rust. `tools/check_tree.py` enforces
that the module tree matches the C# tree exactly: 0 missing, 0 empty, 0 orphan.

---

## 10. Known deviations from the C#

The full list is in the porting notes. These are the ones that will bite you.

### 10.1 The `CMWebSocket` HTTP client purpose does not reach the transport

The C#'s `CMClient.CreateConnection` hands the `WebSocketConnection` the HTTP client of the
`CMWebSocket` purpose, so a user-supplied proxy, TLS configuration or header set applies to
the CM websocket. The Rust port opens the socket with `tokio_tungstenite::connect_async`,
which performs its own handshake, so a user-supplied HTTP client is **ignored** for this
transport. The `HttpClientPurpose::CMWebSocket` variant exists but is unused by the default
factory.

### 10.2 The websocket URI needs the host name

The C# builds the URI from the `EndPoint`, and a `DnsEndPoint` keeps the host name — which
is also the TLS server name. `SUMMARY.md` §8 freezes `IConnection::connect` to take a
`SocketAddr`, which has lost the host name, so the port carries the host name alongside the
server record and uses it for the URI. This is why the `ConnectionFactory` takes the record.

### 10.3 Event ordering

A C# `event` invokes handlers synchronously, in registration order, and a handler sees every
event. The connection events reach the library through one ordered, unbounded stream per
subscriber (`ConnectionEvents::events`), which keeps "every event, in order" but not "the
raiser waits for the handler". The per-event broadcast receivers that are still offered can
lose events and are not ordered relative to each other. See [§3.3](#33-the-async-model) and
the porting notes §14.

### 10.4 `event` → `broadcast` for the *outbound* path

Same root cause, same caveat.

### 10.5 Unknown protobuf fields are dropped

`prost` does not preserve unknown fields, so a message that a newer Steam server extends
will lose the extension on a decode/encode round trip. The C#'s protobuf-net preserves them.
This is inherent to the prost approach and is the port's most significant fidelity gap.

### 10.6 Non-ASCII console output

An earlier revision of this guide claimed the samples print non-ASCII as `?` because
`println!` writes UTF-8 bytes that the console decodes with its OEM code page. Rust's std
does not do that: on a console handle it converts to UTF-16 and calls `WriteConsoleW`, so
the output code page is not involved. The C# is the side that encodes into the console code
page. Both render the block characters; only a font without the glyphs shows boxes.
the porting notes §17.2 has the full comparison.

### 10.7 macOS and Windows-only machine info

The machine-ID collection has per-platform implementations. The macOS (IOKit) and Windows
(WMI) paths are ported; the Linux path falls back to a generic implementation.

### 10.8 DNS is resolved eagerly

The C# keeps a lazy `DnsEndPoint` and resolves at connect time. The port resolves the host
when the server record is created, because `SUMMARY.md` §8 requires connections to be
made to a `SocketAddr`. This changes *when* a DNS failure surfaces, not whether it does.

### 10.9 The transport order is TCP, websocket, UDP — not websocket, TCP, UDP

The C# `CMClient.CreateConnection` prefers the **websocket** transport, then TCP, then UDP. The Rust
port uses the same order. An earlier revision preferred TCP because the websocket session appeared to
die after a minute or two; the live runs showed those were transient `TryAnotherCM` responses, and a
5-minute soak over the default transport passes (the porting notes §22).

---

## 11. Building, running, verifying

### 11.1 The RAM and disk policy — read this first

This machine has 32 GB of RAM and was bricked once by this project. The cause: every
subagent used its **own** `CARGO_TARGET_DIR`, so N parallel `cargo check`s each recompiled
the entire dependency graph, each spawning up to 20 `rustc` processes. Eight target
directories and ~40 GB of artefacts later, the machine was unresponsive.

**The rules:**

1. **One** target directory — the default `target/`. Never set `CARGO_TARGET_DIR` for a
   normal build.
2. `CARGO_BUILD_JOBS = 2` for every cargo invocation.
3. **Never two cargo runs at once**, workspace-wide. Cargo serialises on the target
   directory's lock, so a second invocation blocks rather than corrupting — but it is still
   a second cargo.
4. Subagents must not run cargo while the integrator is running it.
5. The incremental compilation cache is **disabled** (`incremental = false` in the workspace
   `Cargo.toml`). It grew to 33 GB on a drive that had 33 GB free, and it bought nothing
   here. Do not turn it back on.

**Measure before committing to a build.** If the resident memory of the compiler climbs
past roughly 8 GB, stop and drop `CARGO_BUILD_JOBS` to 1:

```powershell
Get-Process rustc -ErrorAction SilentlyContinue |
    Measure-Object -Property WorkingSet64 -Sum |
    ForEach-Object { "rustc RAM: {0} MB" -f [math]::Round($_.Sum/1MB) }
```

### 11.2 Build

```powershell
cd <the repository root>
$env:CARGO_BUILD_JOBS = 2
cargo build --workspace
```

### 11.3 The four verification gates

These are the automated checks that the port still matches the C#. Run all four.

> **Not present in this tree.** The four scripts below live in the original working tree;
> this snapshot contains only `crates/steamkit-tools/tools/gen_proto_registry.py`. The
> descriptions are kept because they define what each gate checks. The gates that do exist
> here are `cargo check --workspace` and `cargo test --workspace --no-fail-fast`.

```powershell
python tools\verify_steam_language.py   # 91 enums / 3093 variants, 0 mismatches
python tools\verify_wire_format.py       # 145 fields compared, 0 width mismatches
python tools\check_tree.py              # 0 missing, 0 empty, 0 orphan
python tools\audit_mods.py              # every mod.rs matches the files on disk
```

| Script | What it asserts |
|---|---|
| `verify_steam_language.py` | every generated Rust enum matches the C# enum — same name, same value, same variant set |
| `verify_wire_format.py` | parses the C# `bw.Write(...)` calls per struct and compares the widths against the Rust field kinds |
| `check_tree.py` | the Rust module tree matches the C# directory tree, with no empty or orphaned files |
| `audit_mods.py` | every `mod.rs` declares exactly the files that are on disk |

The guide has its own checkers, so it cannot silently rot:

```powershell
python tools\validate_guide.py        # encoding, structure, fences, tables, anchors, coverage
python tools\render_test_guide.py     # renders with markdown-it, python-markdown and mistune
python tools\check_guide_coverage.py  # every hand-written module has an entry
python tools\check_guide_line_counts.py  # every stated line count matches the file
```

`validate_guide.py` checks the encoding (valid UTF-8, no mojibake, no control or invisible
characters), the code fences, the table column counts, the heading hierarchy, the table of
contents against the actual heading anchors, and that every path the guide mentions exists
in the bundle. `render_test_guide.py` parses the document with three independent Markdown
engines and checks that the resulting HTML is balanced.

`tools/` also holds the maintenance scripts: `stage_protos.py` (re-stage Valve's protos),
`regen_mods.py` (regenerate `mod.rs` files), `make_module_tree.ps1` (scaffold the tree from
the C# tree), `fix_mods.py`, and `sync_guide_line_counts.py` (refresh the `NNN lines` figure
of every per-file entry in the guide's source parts — the same transformation
`assemble_guide.py` applies to the joined body before it writes the guide).

### 11.4 The test suite

```powershell
$env:CARGO_BUILD_JOBS = 2
cargo test --workspace --no-fail-fast
```

Expect **458 passed, 0 failed, 4 ignored**.

**Always use `--no-fail-fast`.** Without it, cargo stops at the first failing test binary
and hides every failure after it. That is not a hypothetical: an earlier run of this project
reported "4 failed" when the real number was 8, because the run had stopped at the first
failure.

The five ignored tests are ignored on purpose and each says why in its source:
two in `cdn_client.rs` need a mock HTTP transport that `reqwest` cannot be given without a
new dependency; the `simple_console_debug_listener` child-process helper; the live
`depot_e2e` test, which talks to Steam; and one rustdoc doctest.

### 11.5 Running the samples

```powershell
cargo run -p steamkit-samples --bin 000_authentication -- <username> <password>
```

**Do not put a real password on the command line.** It lands in the process list, where any
other process on the machine can read it. Use the QR flow instead — it needs no credential
at all:

```powershell
cargo run -p steamkit-samples --bin 001_authentication_with_qr_code
```

It prints a challenge URL as ASCII art; scan it with the Steam Mobile app and approve the
login. This is the recommended way to exercise a live login.

### 11.6 The dev tools

```powershell
# regenerate the 91 Valve enums from the .steamd definitions
cargo run -p steamkit-tools --bin steamkit-lang-gen -- <input> <output>

# regenerate the prost protobuf code
cargo run -p steamkit-tools --bin steamkit-proto-gen

# dump the protobuf definitions
cargo run -p steamkit-tools --bin steamkit-proto-dump -- --protos crates\steamkit-protos\proto\ <out>

# read the capture files that the NetHook2 DLL writes
cargo run -p steamkit-tools --bin steamkit-nethook-analyzer -- <capture-dir> --list
cargo run -p steamkit-tools --bin steamkit-nethook-analyzer -- <capture-dir> --all
```

---

## 12. The bug history

Twelve real defects, all of them found *after* the port compiled cleanly. Read this section
before you assume your own change is safe.

| # | Where | What was wrong |
|---|---|---|
| 1 | `util/crypto_helper.rs` | `symmetric_decrypt` allocated a **zeroed** buffer and never copied the ciphertext into it. The function had never decrypted anything. |
| 2 | `steam/cdn/depot_chunk.rs` | the same zeroed-buffer bug, **and** the IV was decrypted from the zeroed `iv` instead of `data[..16]`. |
| 3 | `util/lzma_decoder.rs` | `bit - 1` on a `u32` **panics**; the C# computes it on a `uint`, which wraps. Every LZMA decode was broken. |
| 4 | `types/depot_manifest.rs` | three bugs in `decrypt_name`: the AES key and IV were passed to `new_from_slices` **reversed**; strict base64 rejected the trailing newline the C# ignores; the null trim checked the byte *past* the plaintext. |
| 5 | `base/generated/steam_language_internal.rs` | the wire macro captured the field kind as `$kind:ty` and tried to re-match it against literal tokens. A `ty` fragment is opaque — **no spelling of it can work**. |
| 6 | `steam/discovery/smart_cm_server_list.rs` | the HTTP client factory was invoked while the configuration was being built. The C# invokes it when the server list is actually fetched. |
| 7 | `types/key_value.rs` | the text reader only stripped a UTF-8 BOM. The C#'s `StreamReader` also detects UTF-16 LE and BE. |
| 8 | `types/steam_id.rs` | `render(false)` delegated to the Steam3 form for every account type but `Invalid`/`Individual`. The C#'s `Convert.ToString( this )` binds to the **`ulong`** overload, so it renders the numeric id. |
| 9 | `util/lzma_util.rs` | `try_decompress` handed `decompress` a *pre-filled* buffer, and a `Vec` that is written to grows at its end, so the decoded data landed at an offset. The C# writes into a `MemoryStream` that has the capacity but **no content**. |
| 10 | `util/lzma/compress/lzma/lzma_encoder.rs` | `baseVal - posSlot - 1` underflowed for `posSlot == 4`. The C# computes it on a `UInt32`, which wraps to `0xFFFFFFFF`, and its bit tree then wraps the index back to `0`. |
| 11 | `steam/cm_client.rs` | the WebSocket transport the port already shipped was **never wired up**, and the order was TCP/UDP/WebSocket instead of the C#'s WebSocket/TCP/UDP. A WebSocket-only CM server — which is what the default fallback list holds first — could not connect at all. |
| 12 | `steamkit-tools/nethook/message_reading.rs` | the reader stripped the message number and then let the header read it *again*. The analyzer could not decode a single real capture. |

**Defect 11 has a sequel.** The websocket transport was wired up, but the first runs appeared to fail
before a logon. The live runs showed those were transient `TryAnotherCM` responses; the port now uses
the C# order (websocket first), and a 5-minute soak passes (the porting notes §22).

**The pattern.** Nine of the twelve are in cryptography, wire format or framing — the areas
everyone assumes are mechanical. Three of the twelve are C# semantics that Rust does not
share: unsigned wraparound, `Convert.ToString` overload resolution, and the difference
between a stream with capacity and a buffer with length.

**The lesson.** When porting, the C# is the specification, and the specification includes
its arithmetic model, its overload resolution, and its "an empty stream is not an empty
buffer" conventions. None of those survive a mechanical translation.

---

Two more were found by capturing the port's traffic against the C# reference's, field by
field, on the live server:

| # | Where | What was wrong |
|---|---|---|
| 13 | `util/utils.rs` | `detect_windows_os_type` never parsed the Windows version. It took the text between the brackets of `cmd /c ver` and stripped them, but not the literal `Version ` prefix inside them, so the version parser always failed and the OS type was always `Windows10`. The C# reports `Win11` (20) on Windows 11 |
| 14 | `util/hardware_utils.rs` | The `machine_id` component `FF2` is the SHA1 of the combined MAC address of the network adapters, and the adapters were enumerated in a different order than the C# enumerates them, so the hash differed. The port now uses the C#'s own enumeration - `NetworkInterface.GetAllNetworkInterfaces()`, filtered by the registry `Connection\PnpInstanceID` starting with `PCI` - so the two produce the same value |

Twelve more were found on 2026-10-22 by a review of the networking layer and the CM client
lifecycle, neither of which the ported C# suite tests. Each has a test in one of the seven
Rust-only test files of [§7.2.8](#728-the-rust-only-tests-of-the-networking-layer-and-the-cm-client-lifecycle),
and each of those tests was seen to fail with the defect put back.

| # | Where | What was wrong |
|---|---|---|
| 15 | `networking/steam3/i_connection.rs`, `steam/cm_client.rs` | The connection events travelled on broadcast channels of 64 slots, and the CM dispatch loop awaits every handler. A burst of more than 64 messages while a handler was busy made the loop skip `RecvError::Lagged` and **silently lose messages** - job responses that never arrived, requests that timed out. The library now consumes one ordered, unbounded stream per connection |
| 16 | `steam/cm_client.rs` | The dispatch loop `select!`ed over three independent channels, so a `Disconnected` could be acted on before the messages that arrived ahead of it |
| 17 | `networking/steam3/envelope_encrypted_connection.rs` | `Connected` and the messages were handled by two different tasks. The server sends `ChannelEncryptRequest` right behind the socket opening; when the message task ran first, the request was rejected as arriving in the `Disconnected` state and the handshake stalled until the connection timed out |
| 18 | `networking/steam3/tcp_connection.rs` | `connect` started the read loop before it raised `Connected`, so on a multi-threaded runtime the server's first message, or a `Disconnected`, could be raised first |
| 19 | `steam/cm_client.rs` | A session the server closed was moved to another server even when the client **expected** the disconnection - so `SteamUser::log_off` reconnected and **logged the user back on** |
| 20 | `steam/cm_client.rs` | The remembered `ClientLogon` was never forgotten: a **refused logon (a wrong password) was retried** on server after server, a session logged off with `LoggedInElsewhere` logged back on (kicking the other session), and a later `connect` repeated the previous account's logon unasked. There was also no limit on the moves |
| 21 | `steam/cm_client.rs` | When the move to another server after a server-side disconnect failed, **no disconnection was ever announced** - `switch_session` had already emptied the connection slot, so `handle_disconnected` returned early - and `is_connected` stayed `true` |
| 22 | `steam/cm_client.rs` | A failed `connect` left its dispatch loop running. With the WebSocket transport, which also raises `Disconnected` for a failed attempt, that loop could announce the failure a second time or start a move to another server |
| 23 | `steam/cm_client.rs` | A dispatch loop superseded by a later `connect` could take the newer session's connection out of the client and tear it down. Every loop now serves one generation and touches the client only while it is current |
| 24 | `steam/cm_client.rs` | A malformed message inside a `Multi` ended the dispatch loop **without disconnecting**, leaving the connection open with nobody reading it. The C# disconnects |
| 25 | `networking/steam3/envelope_encrypted_connection.rs` | The forwarding tasks held the connection strongly through a reference cycle, so every connection ever made leaked its tasks and buffers |
| 26 | `networking/steam3/envelope_encrypted_connection.rs` | The handshake carried its own copy of the universe keys, and its **`Dev` key was corrupt** (146 bytes instead of 160): a `Dev` universe handshake could never succeed. It now reads `KeyDictionary` |

One hardening came with them: the channel filter's HMAC check
(`net_filter_encryption_with_hmac.rs`) now compares in constant time.

**The pattern, again.** None of the twelve was visible to a test that runs one message
through a quiet connection: they need bursts, races, a second session, or a session that
ends in an unusual way. The C# does not have most of them because its events are
synchronous and its `CMClient` never reconnects by itself - the defects are in the places
where the port's design differs from the C#'s, not where it follows it.

## 13. The original C# repository

`csharp-original/` is a **snapshot of the upstream SteamKit2 working tree**, byte-for-byte
what this commit checks out:

```
94812dc360304c15a3e901dcaae3f92715bdfb37
Mon Nov 2 16:48:07 2026 +0000 Bump version to 4.0.0-Beta.1
```

```
csharp-original/
├── SteamKit2/          ← the library, the tests, the 14 samples
├── Resources/          ← the tooling: NetHook2 (C++), NetHookAnalyzer2, ProtobufGen, SteamLanguageParser
├── Samples/            ← the 14 sample projects
├── PINNED_COMMIT.txt   ← the upstream remote, the branch, the commit, and how to fetch it
└── SteamKit.slnx
```

The version-control history is **not** included — it was 64 MB of git objects, and this
directory is meant to hold the source and the documentation. `PINNED_COMMIT.txt` records
where the tree came from and how to get the history back.

Use it as the specification. When the port and the C# disagree, the C# is right — unless
the porting notes says otherwise, in which case that file records why.

To get the history, or to read a file at the ported commit:

```powershell
git clone https://github.com/SteamRE/SteamKit.git
cd SteamKit
git checkout 94812dc360304c15a3e901dcaae3f92715bdfb37
git show 94812dc3:SteamKit2/SteamKit2/Types/SteamID.cs
```

---

## 14. Where to go next

1. **Deeper use of a logged-on session.** Logging on works (QR flow, over TCP), and
   `900_live_check` showed the basics of a logged-on session working against live Steam:
   the account info, the friends list, a unified message, PICS product info and a depot
   key ([§15.1](#151-verified)). Not yet exercised live: downloading from the CDN with that
   key, the game coordinators (sample `022`), chat and persona changes, and a session that
   stays up for hours rather than a minute.
2. **The websocket CM transport is verified.** A 5-minute soak over the default transport answered a
   probe with 0 server moves (the porting notes §22).
3. **The remaining transport deviation** — the `CMWebSocket` purpose. The console encoding
   item is closed: it was a documentation error (see 10.6).

**Two items need a person, not a code change**, and they are listed in
[§15.2](#152-what-still-needs-a-person):

* scanning a QR challenge with the Steam mobile app for any further live login — the
  only way to log on from this port without a password on the command line;
* injecting the `NetHook2` DLL into a live `Steam.exe` — the Steam client is not installed
  on this machine.

### 14.1 What has and has not been exercised

| Area | Status |
|---|---|
| `Resources/ProtobufGen` → a Rust proto→Rust generator | ported; **exercised** — it regenerates `steamkit-protos` |
| `Resources/ProtobufDumper` | ported; **exercised** — `--protos` round-trips the 72 vendored `steam/` protos (72 in, 72 out, stable) |
| `Resources/NetHookAnalyzer2` | ported to a CLI; **exercised** — it decodes a real capture produced by the port itself |
| `Resources/NetHook2` | ported to a `cdylib`; the log format is proven consistent with the analyzer, but the DLL has **never been injected into a real `Steam.exe`** — see [§15.2](#152-what-still-needs-a-person) |
| `SteamKit2/Tests` | 33 of 34 files ported; `Helpers/*` folded into `tests/common/mod.rs`; plus seven Rust-only files (§7.2.8). **458 pass, 0 fail, 4 ignored** |
| Live-network tests | 4 `#[ignore]`d (documented inline) |
| The CM protocol over TCP | **exercised** — the server answers the logon with a `ClientLogOnResponse`; see [§15](#15-what-has-been-verified-against-live-steam) |
| The CM protocol over the websocket | **not working** — the server does not answer the hello |
| A credentialed Steam login | **exercised** (2026-10-22) — QR flow over TCP, logon accepted, logoff ended the session; see [§15.1](#151-verified) |
| A logged-on session | **exercised** (2026-10-23) — `900_live_check`: SteamID, account info, friends list, a unified message, PICS product info, a depot key, 60 s idle; 9 of 9 checks passed |

**The port has logged on to a real Steam account and used the session**
([§15](#15-what-has-been-verified-against-live-steam)). What is left unknown is the deeper
use of it — the CDN, the game coordinators, and long-lived sessions.

the porting notes has every deviation with its reason. the porting contract has the binding
rules and the frozen type signatures. the handoff notes is the short "where are we now" note.
`FILE_INDEX.md` maps every module to the line of this guide that documents it.

---

## 15. What has been verified against live Steam

This section is the record of what the port has actually been shown to do against the real
Steam servers, and — just as importantly — what it has not.

### 15.1 Verified

**The whole CM flow, over TCP.** The port completes the complete CM exchange with a real
Steam CM server:

```
client → server   ChannelEncryptRequest / ChannelEncryptResponse   (the handshake)
client → server   ClientHello            protocol_version = 65581
client → server   ClientLogon            269 bytes
server → client   ClientLogOnResponse    33 bytes
```

**The `ClientLogon` is byte-for-byte the size the C# reference sends** — 269 bytes — and the
server answers the Rust port exactly as it answers the C#. That was established by capturing
the traffic of **both** implementations with the **same** `DebugNetworkListener` mechanism and
comparing the two captures side by side. The C#'s own `ClientLogOnResponse` is 38 bytes
against the Rust port's 33; the two bodies differ, the exchange does not.

**The client moves to another CM server by itself.** When Steam answers a logon with
`EResult.TryAnotherCM` or `EResult.ServiceUnavailable`, the port marks the server bad,
switches the connection to the next candidate from `SmartCMServerList`, and repeats the logon
on the new server. The C# marks the server bad and leaves the reconnection to the caller. The
implementation is in the `cm_client.rs` entry; the porting notes §12 has the ledger entry.

**A credentialed login, with the QR flow (2026-10-22).** Sample `001_authentication_with_qr_code`
connected over TCP, began a QR authentication session, and was approved from the Steam
mobile app on the developer's own account. `polling_wait_for_result` returned the account
name and a refresh token, `log_on` with that token was answered `EResult::OK`
("Successfully logged on!"), and the sample's `log_off` then ended the session. The run
also confirmed two of the fixes of §12 against the real server:

* **The logoff was not undone** (defect 19). The session closed and stayed closed — no
  reconnection, no second logon — where the client used to move to another server and log
  the user straight back on.
* **The moves to another server work before a logon.** Before the code was scanned, Steam
  answered `TryAnotherCM` / `ServiceUnavailable` three times in about four minutes; the
  client moved each time without announcing a disconnection, and the QR flow carried on
  on the new server.

The run used sample `001` as it was before the changes described in its entry (§7.1); the
library code was the code this guide documents. A second run on 2026-10-23, with the changed
sample, logged on, logged off and exited on its own within a second of the scan. The code
was scanned 21 seconds after it appeared, so Steam did not move that session and the
sample's one-QR-session-at-a-time path did not run.

**A logged-on session does its work (2026-10-23).** The Rust-only `900_live_check` logged
on with the QR flow and ran its nine checks against live Steam; all nine passed:

```
PASS  Logon                        accepted
PASS  SteamID                      assigned, an individual account
PASS  Account info                 received, with a persona name
PASS  Friends list                 4 entries, 3 of them friends
PASS  Unified message              Player.GetGameBadgeLevels: player level 6
PASS  PICS product info            app 730 is "Counter-Strike 2"
PASS  Depot key                    depot 731: 32 byte key received (not shown)
PASS  Session stays up             60 seconds, 0 server move(s)
PASS  Log off                      ended the session, and it stayed ended
```

The whole run took 68 seconds from the approval to the exit. That covers the job-based
requests (`SteamApps`, through `AsyncJobManager`), the unified-message path
(`SteamUnifiedMessages`), and the callbacks Steam pushes after a logon. An idle session
stayed up for a minute; that is too short to prove the heartbeat against the server's
timeout.

**`021_web_api` really fetches from the live Steam Web API.** The sample takes no
credentials, calls `ISteamNews.GetNewsForApp` for TF2 (app 440), Dota 2 (570) and CS:GO
(730), and prints the news it gets back — including the headlines that carry **non-ASCII
characters**. The characters still reach the console as `?` (the console-encoding deviation
of §10.6), but the data itself is real and complete.

**The 26 real depot manifests are parsed byte-identically to the C# reference.** The manifests
that `DepotDownloaderMod` downloaded are parsed by the port's `types/depot_manifest.rs` and
come out byte-for-byte the same as the C# `SteamKit2` produces. This covers the decryption
(`decrypt_name`, the AES key and IV, the base64 handling) and the binary serialisation
round trip.

### 15.2 What still needs a person

Two items cannot be closed by running the code on this machine.

**The QR scan.** Sample `001_authentication_with_qr_code` prints a challenge URL as ASCII art
(and as an SVG) that has to be scanned with the Steam mobile app and approved there. It is
the only way to log on from this port without a password on the command line, and it needs
a person with the app on a phone. `001` and `900_live_check` both use it; every further
live test of a logged-on session needs a fresh scan.

**The `NetHook2` injection.** The `NetHook2` DLL is ported, it compiles, and its capture
format is proven consistent with the port's own `steamkit-nethook-analyzer` — the analyzer
decodes a real capture produced by the port. But **the Steam client is not installed on this
machine**, so the DLL could not be injected into a live `Steam.exe`. The injector and the
trampoline are therefore still unproven on a live process.

### 15.3 What is still unproven

Two things are not settled, and neither is a known defect.

**The CM session is closed by the server after a few minutes of an unauthenticated QR
session.** The port receives a fresh challenge every thirty seconds for several minutes, so
the polls are reaching Steam and being answered, and then the server closes the connection.
The messages that have been compared - `ClientHello`, `ClientLogon`, and the
authentication poll - are identical to the C# reference's, and the poll uses the same
`EMsg` and the same RPC name (`Authentication.PollAuthSessionStatus#1`) as the C#'s
`SteamUnifiedMessages`, so the cause is still open. It no longer stops a login: in the
2026-10-22 run Steam told the idle session to use another server three times in about four
minutes (whether this is the same behaviour is not established — that run did not record
which of `TryAnotherCM` / `ServiceUnavailable` it was, nor capture the traffic). The client
moved each time, the QR flow carried on, and the login succeeded (§15.1).

**The `NetHook2` DLL has never been injected into a live `Steam.exe`.** The Steam client is
not installed on the machine the port was developed on. Everything the injector does before
the hooking is now covered by tests that run against the live machine - it finds a process
by name, reads a module path, checks the architecture, and parses a command line - so the
part that has never run is the trampoline itself.
