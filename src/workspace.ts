import type { ParserMode } from "./types";
export type View =
  "Explore" | "Evidence" | "Statistics" | "Parsers" | "Settings";
export const parsers: {
  mode: ParserMode;
  name: string;
  description: string;
  formats: string;
}[] = [
  {
    mode: "auto",
    name: "Auto detect",
    description:
      "Recognizes access records and JSON objects. Unknown lines remain searchable as raw text with Unparsed status.",
    formats: "Apache / Nginx Combined · JSON Lines · raw text",
  },
  {
    mode: "apache",
    name: "Apache Access",
    description:
      "Validates Common and Combined access records. Field validation failures retain recoverable fields as Partial; pattern mismatches remain Unparsed.",
    formats: "Common Log Format · Combined Log Format",
  },
  {
    mode: "nginx",
    name: "Nginx Access",
    description:
      "Reads standard Common/Combined access layouts. Choose this reader to identify Nginx explicitly; its Combined format also matches Apache.",
    formats: "Standard access log · Combined",
  },
  {
    mode: "json_lines",
    name: "JSON Lines",
    description:
      "Reads one JSON object per line, retains original fields and extracts documented HTTP/date aliases. Invalid lines remain visible with diagnostics.",
    formats: "JSONL · NDJSON · UTF-8 objects",
  },
  {
    mode: "syslog",
    name: "Syslog",
    description:
      "Reads RFC 5424 version 1 and BSD HOST TAG[PID]: MSG files. BSD dates retain their original text without inventing a year or timezone.",
    formats: "RFC 5424 · BSD file layout · one message per line",
  },
  {
    mode: "iptables",
    name: "Netfilter / iptables",
    description:
      "Extracts Netfilter LOG fields, addresses, ports and protocol from bare or Syslog-wrapped records. Prefixes remain evidence; no firewall verdict is inferred.",
    formats: "IN / OUT / SRC / DST / PROTO · IPv4 / IPv6",
  },
  {
    mode: "haproxy",
    name: "HAProxy HTTP",
    description:
      "Reads standard option httplog records with optional captured headers and Syslog envelopes. Request dates without an explicit UTC offset remain undated with a diagnostic.",
    formats: "HTTP log · timers · frontend / backend / server",
  },
  {
    mode: "modsecurity",
    name: "ModSecurity audit",
    description:
      "Groups native ModSecurity 2.x A–Z transactions, up to 1 MiB per event. Extracts request, response and trailer fields; incomplete transactions remain visible with diagnostics.",
    formats:
      "Native serial audit · multiline · matching transaction boundaries",
  },
  {
    mode: "text",
    name: "Plain text",
    description:
      "Reads every line without structured extraction. A successfully read line is Parsed; encoding warnings are reported separately.",
    formats: "Uncompressed text · original source order",
  },
];
