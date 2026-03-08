/*
 * fyaml-tokenize: reads YAML from stdin and outputs tokens or events as JSON lines.
 *
 * Uses libfyaml's low-level scanner API (fy_scan) to produce token-level
 * output, or the high-level parser API (fy_parser_parse) for event-level
 * output when --events is passed.
 *
 * Modes:
 *   (default)       Read all of stdin, process once, exit.
 *   --batch         Length-prefixed protocol: read "<len>\n<bytes>" repeatedly,
 *                   write JSON lines + "---END\n" per input. Errors go to
 *                   stdout (not stderr) so the reader can associate them.
 *   --events        Event mode (parser API) instead of token mode (scanner API).
 *
 * Build: see accompanying Makefile
 */

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// cref: libfyaml public API
#include <libfyaml.h>

// cref: libfyaml internal — needed for fy_parser_set_default_document_state()
// to initialize default tag directives (!! → tag:yaml.org,2002:) before scanning.
// Without this, fy_fetch_tag() fails on shorthand tags because current_document_state is NULL.
#include "fy-parse.h"

/* Map fy_token_type enum to string name. Only YAML token types. */
// cref: enum fy_token_type (FYTT_STREAM_START .. FYTT_SCALAR)
static const char *token_type_name(enum fy_token_type type)
{
	switch (type) {
	case FYTT_NONE:                 return "NONE";
	case FYTT_STREAM_START:         return "STREAM_START";
	case FYTT_STREAM_END:           return "STREAM_END";
	case FYTT_VERSION_DIRECTIVE:    return "VERSION_DIRECTIVE";
	case FYTT_TAG_DIRECTIVE:        return "TAG_DIRECTIVE";
	case FYTT_DOCUMENT_START:       return "DOCUMENT_START";
	case FYTT_DOCUMENT_END:         return "DOCUMENT_END";
	case FYTT_BLOCK_SEQUENCE_START: return "BLOCK_SEQUENCE_START";
	case FYTT_BLOCK_MAPPING_START:  return "BLOCK_MAPPING_START";
	case FYTT_BLOCK_END:            return "BLOCK_END";
	case FYTT_FLOW_SEQUENCE_START:  return "FLOW_SEQUENCE_START";
	case FYTT_FLOW_SEQUENCE_END:    return "FLOW_SEQUENCE_END";
	case FYTT_FLOW_MAPPING_START:   return "FLOW_MAPPING_START";
	case FYTT_FLOW_MAPPING_END:     return "FLOW_MAPPING_END";
	case FYTT_BLOCK_ENTRY:          return "BLOCK_ENTRY";
	case FYTT_FLOW_ENTRY:           return "FLOW_ENTRY";
	case FYTT_KEY:                  return "KEY";
	case FYTT_VALUE:                return "VALUE";
	case FYTT_ALIAS:                return "ALIAS";
	case FYTT_ANCHOR:               return "ANCHOR";
	case FYTT_TAG:                  return "TAG";
	case FYTT_SCALAR:               return "SCALAR";
	default:                        return "UNKNOWN";
	}
}

/* Map fy_event_type enum to string name. */
// cref: enum fy_event_type (FYET_STREAM_START .. FYET_ALIAS)
static const char *event_type_name(enum fy_event_type type)
{
	switch (type) {
	case FYET_NONE:           return "None";
	case FYET_STREAM_START:   return "StreamStart";
	case FYET_STREAM_END:     return "StreamEnd";
	case FYET_DOCUMENT_START: return "DocumentStart";
	case FYET_DOCUMENT_END:   return "DocumentEnd";
	case FYET_MAPPING_START:  return "MappingStart";
	case FYET_MAPPING_END:    return "MappingEnd";
	case FYET_SEQUENCE_START: return "SequenceStart";
	case FYET_SEQUENCE_END:   return "SequenceEnd";
	case FYET_SCALAR:         return "Scalar";
	case FYET_ALIAS:          return "Alias";
	default:                  return "Unknown";
	}
}

/* Print a JSON-escaped version of str (length len) to fp. */
static void json_escape(FILE *fp, const char *str, size_t len)
{
	for (size_t i = 0; i < len; i++) {
		unsigned char c = (unsigned char)str[i];
		switch (c) {
		case '"':  fputs("\\\"", fp); break;
		case '\\': fputs("\\\\", fp); break;
		case '\b': fputs("\\b", fp);  break;
		case '\f': fputs("\\f", fp);  break;
		case '\n': fputs("\\n", fp);  break;
		case '\r': fputs("\\r", fp);  break;
		case '\t': fputs("\\t", fp);  break;
		default:
			if (c < 0x20) {
				fprintf(fp, "\\u%04x", c);
			} else {
				fputc(c, fp);
			}
			break;
		}
	}
}

/* Read all of stdin into a malloc'd buffer. Sets *out_len. */
static char *read_stdin(size_t *out_len)
{
	size_t cap = 4096;
	size_t len = 0;
	char *buf = malloc(cap);
	if (!buf) return NULL;

	for (;;) {
		size_t n = fread(buf + len, 1, cap - len, stdin);
		len += n;
		if (n == 0) break;
		if (len == cap) {
			cap *= 2;
			char *tmp = realloc(buf, cap);
			if (!tmp) { free(buf); return NULL; }
			buf = tmp;
		}
	}
	*out_len = len;
	return buf;
}

/*
 * Process a single YAML input buffer and write results to out_fp.
 * In batch mode, errors go to out_fp (stdout) instead of stderr.
 * Returns 0 on success (STREAM_END seen), 1 on error.
 */
static int process_input(const char *input, size_t input_len,
                         int events_mode, FILE *out_fp, FILE *err_fp)
{
	struct fy_parse_cfg cfg = {
		.flags = FYPCF_QUIET,
		.search_path = NULL,
		.userdata = NULL,
		.diag = NULL,
	};
	struct fy_parser *fyp = fy_parser_create(&cfg);
	if (!fyp) {
		fprintf(err_fp, "{\"error\":\"fy_parser_create failed\"}\n");
		return 1;
	}

	int rc = fy_parser_set_string(fyp, input, input_len);
	if (rc != 0) {
		fprintf(err_fp, "{\"error\":\"fy_parser_set_string failed\"}\n");
		fy_parser_destroy(fyp);
		return 1;
	}

	int error = 0;
	int saw_stream_end = 0;

	if (events_mode) {
		struct fy_event *fye;
		while ((fye = fy_parser_parse(fyp)) != NULL) {
			const char *type_str = event_type_name(fye->type);

			fprintf(out_fp, "{\"type\":\"%s\"", type_str);

			switch (fye->type) {
			case FYET_DOCUMENT_START:
				fprintf(out_fp, ",\"implicit\":%s",
					fye->document_start.implicit ? "true" : "false");
				break;
			case FYET_DOCUMENT_END:
				fprintf(out_fp, ",\"implicit\":%s",
					fye->document_end.implicit ? "true" : "false");
				break;
			case FYET_SCALAR: {
				size_t len = 0;
				const char *text = fy_token_get_text(fye->scalar.value, &len);
				if (text) {
					fprintf(out_fp, ",\"value\":\"");
					json_escape(out_fp, text, len);
					fprintf(out_fp, "\"");
				} else {
					fprintf(out_fp, ",\"value\":\"\"");
				}
				if (fye->scalar.anchor) {
					size_t alen = 0;
					const char *atext = fy_token_get_text(fye->scalar.anchor, &alen);
					if (atext) {
						fprintf(out_fp, ",\"anchor\":\"");
						json_escape(out_fp, atext, alen);
						fprintf(out_fp, "\"");
					} else {
						fprintf(out_fp, ",\"anchor\":null");
					}
				} else {
					fprintf(out_fp, ",\"anchor\":null");
				}
				if (fye->scalar.tag) {
					size_t tlen = 0;
					const char *ttext = fy_token_get_text(fye->scalar.tag, &tlen);
					if (ttext) {
						fprintf(out_fp, ",\"tag\":\"");
						json_escape(out_fp, ttext, tlen);
						fprintf(out_fp, "\"");
					} else {
						fprintf(out_fp, ",\"tag\":null");
					}
				} else {
					fprintf(out_fp, ",\"tag\":null");
				}
				break;
			}
			case FYET_ALIAS: {
				if (fye->alias.anchor) {
					size_t alen = 0;
					const char *atext = fy_token_get_text(fye->alias.anchor, &alen);
					if (atext) {
						fprintf(out_fp, ",\"name\":\"");
						json_escape(out_fp, atext, alen);
						fprintf(out_fp, "\"");
					} else {
						fprintf(out_fp, ",\"name\":null");
					}
				} else {
					fprintf(out_fp, ",\"name\":null");
				}
				break;
			}
			case FYET_MAPPING_START:
			case FYET_SEQUENCE_START: {
				struct fy_token *anchor_tok = (fye->type == FYET_MAPPING_START)
					? fye->mapping_start.anchor : fye->sequence_start.anchor;
				struct fy_token *tag_tok = (fye->type == FYET_MAPPING_START)
					? fye->mapping_start.tag : fye->sequence_start.tag;
				if (anchor_tok) {
					size_t alen = 0;
					const char *atext = fy_token_get_text(anchor_tok, &alen);
					if (atext) {
						fprintf(out_fp, ",\"anchor\":\"");
						json_escape(out_fp, atext, alen);
						fprintf(out_fp, "\"");
					} else {
						fprintf(out_fp, ",\"anchor\":null");
					}
				} else {
					fprintf(out_fp, ",\"anchor\":null");
				}
				if (tag_tok) {
					size_t tlen = 0;
					const char *ttext = fy_token_get_text(tag_tok, &tlen);
					if (ttext) {
						fprintf(out_fp, ",\"tag\":\"");
						json_escape(out_fp, ttext, tlen);
						fprintf(out_fp, "\"");
					} else {
						fprintf(out_fp, ",\"tag\":null");
					}
				} else {
					fprintf(out_fp, ",\"tag\":null");
				}
				break;
			}
			default:
				break;
			}

			fprintf(out_fp, "}\n");

			if (fye->type == FYET_STREAM_END)
				saw_stream_end = 1;

			fy_parser_event_free(fyp, fye);
		}
	} else {
		/* Token mode — use fy_scan() low-level scanner API */
		rc = fy_parser_set_default_document_state(fyp, NULL);
		if (rc != 0) {
			fprintf(err_fp, "{\"error\":\"fy_parser_set_default_document_state failed\"}\n");
			fy_parser_destroy(fyp);
			return 1;
		}

		struct fy_token *fyt;
		while ((fyt = fy_scan(fyp)) != NULL) {
			enum fy_token_type type = fy_token_get_type(fyt);
			const char *type_str = token_type_name(type);
			if (type == FYTT_STREAM_END)
				saw_stream_end = 1;

			const struct fy_mark *sm = fy_token_start_mark(fyt);
			const struct fy_mark *em = fy_token_end_mark(fyt);

			size_t text_len = 0;
			const char *text = NULL;
			char tag_buf[1024];
			if (type == FYTT_TAG) {
				size_t h_len = 0, s_len = 0;
				const char *handle = fy_tag_token_handle(fyt, &h_len);
				const char *suffix = fy_tag_token_suffix(fyt, &s_len);
				if (h_len == 0 && s_len > 0) {
					text_len = (size_t)snprintf(tag_buf, sizeof(tag_buf),
						"!<%.*s>", (int)s_len, suffix);
				} else {
					text_len = (size_t)snprintf(tag_buf, sizeof(tag_buf),
						"%.*s%.*s", (int)h_len, handle ? handle : "",
						(int)s_len, suffix ? suffix : "");
				}
				text = tag_buf;
			} else if (type == FYTT_TAG_DIRECTIVE) {
				size_t h_len = 0, p_len = 0;
				const char *handle = fy_tag_directive_token_handle(fyt, &h_len);
				const char *prefix = fy_tag_directive_token_prefix(fyt, &p_len);
				text_len = (size_t)snprintf(tag_buf, sizeof(tag_buf),
					"%.*s %.*s", (int)h_len, handle ? handle : "",
					(int)p_len, prefix ? prefix : "");
				text = tag_buf;
			} else if (type == FYTT_SCALAR || type == FYTT_ALIAS ||
			           type == FYTT_ANCHOR || type == FYTT_VERSION_DIRECTIVE) {
				text = fy_token_get_text(fyt, &text_len);
			}

			fprintf(out_fp, "{\"type\":\"%s\",", type_str);
			if (text) {
				fprintf(out_fp, "\"value\":\"");
				json_escape(out_fp, text, text_len);
				fprintf(out_fp, "\",");
			} else {
				fprintf(out_fp, "\"value\":null,");
			}

			if (sm) {
				fprintf(out_fp,
					"\"line\":%d,\"column\":%d,\"offset\":%zu,",
					sm->line, sm->column, sm->input_pos);
			} else {
				fprintf(out_fp,
					"\"line\":null,\"column\":null,\"offset\":null,");
			}

			if (em) {
				fprintf(out_fp,
					"\"end_line\":%d,\"end_column\":%d,\"end_offset\":%zu",
					em->line, em->column, em->input_pos);
			} else {
				fprintf(out_fp,
					"\"end_line\":null,\"end_column\":null,\"end_offset\":null");
			}

			fprintf(out_fp, "}\n");

			fy_scan_token_free(fyp, fyt);
		}
	}

	if (!saw_stream_end) {
		fprintf(err_fp, "{\"error\":\"scan terminated without STREAM_END\"}\n");
		error = 1;
	}

	fy_parser_destroy(fyp);
	return error;
}

int main(int argc, char **argv)
{
	/* Parse argv */
	int events_mode = 0;
	int batch_mode = 0;
	for (int i = 1; i < argc; i++) {
		if (strcmp(argv[i], "--events") == 0) {
			events_mode = 1;
		} else if (strcmp(argv[i], "--batch") == 0) {
			batch_mode = 1;
		}
	}

	if (batch_mode) {
		/*
		 * Batch mode: length-prefixed protocol.
		 * Read "<decimal_length>\n" then exactly that many bytes.
		 * Process each input, write JSON lines + "---END\n".
		 * Errors go to stdout so the reader can associate them.
		 */
		char len_buf[32];
		while (fgets(len_buf, sizeof(len_buf), stdin) != NULL) {
			char *endptr;
			errno = 0;
			unsigned long long raw_len = strtoull(len_buf, &endptr, 10);
			if (errno != 0 || endptr == len_buf || raw_len > (256ULL * 1024 * 1024)) {
				fprintf(stdout, "{\"error\":\"invalid or excessive frame length\"}\n---END\n");
				fflush(stdout);
				continue;
			}
			size_t input_len = (size_t)raw_len;
			char *input = malloc(input_len + 1);
			if (!input) {
				fprintf(stdout, "{\"error\":\"malloc failed\"}\n---END\n");
				fflush(stdout);
				continue;
			}

			size_t total_read = 0;
			while (total_read < input_len) {
				size_t n = fread(input + total_read, 1,
				                 input_len - total_read, stdin);
				if (n == 0) break;
				total_read += n;
			}
			input[total_read] = '\0';

			if (total_read < input_len) {
				fprintf(stdout, "{\"error\":\"short read\"}\n---END\n");
				fflush(stdout);
				free(input);
				continue;
			}

			/* Process — errors go to stdout in batch mode */
			process_input(input, input_len, events_mode, stdout, stdout);
			fprintf(stdout, "---END\n");
			fflush(stdout);
			free(input);
		}
		return 0;
	}

	/* Single-input mode (original behavior) */
	size_t input_len = 0;
	char *input = read_stdin(&input_len);
	if (!input) {
		fprintf(stderr, "{\"error\": \"failed to read stdin\"}\n");
		return 1;
	}

	int error = process_input(input, input_len, events_mode, stdout, stderr);
	free(input);

	return error;
}
