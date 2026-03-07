/*
 * fyaml-tokenize: reads YAML from stdin and outputs tokens as JSON lines.
 *
 * Uses libfyaml's low-level scanner API (fy_scan) to produce token-level
 * output, not event-level. This gives the finest granularity for comparing
 * against yamalgam's Rust scanner.
 *
 * Build: see accompanying Makefile
 */

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

int main(int argc, char **argv)
{
	(void)argc;
	(void)argv;

	/* Read all of stdin */
	size_t input_len = 0;
	char *input = read_stdin(&input_len);
	if (!input) {
		fprintf(stderr, "{\"error\": \"failed to read stdin\"}\n");
		return 1;
	}

	/* Create parser with default config */
	// cref: fy_parser_create()
	struct fy_parse_cfg cfg = {
		.flags = FYPCF_QUIET,
		.search_path = NULL,
		.userdata = NULL,
		.diag = NULL,
	};
	struct fy_parser *fyp = fy_parser_create(&cfg);
	if (!fyp) {
		fprintf(stderr, "{\"error\": \"fy_parser_create failed\"}\n");
		free(input);
		return 1;
	}

	/* Set input from the buffer */
	// cref: fy_parser_set_string()
	int rc = fy_parser_set_string(fyp, input, input_len);
	if (rc != 0) {
		fprintf(stderr, "{\"error\": \"fy_parser_set_string failed\"}\n");
		fy_parser_destroy(fyp);
		free(input);
		return 1;
	}

	/* Initialize default document state so the scanner can resolve
	   shorthand tag handles (! → !, !! → tag:yaml.org,2002:).
	   Without this, fy_fetch_tag() fails because current_document_state is NULL. */
	// cref: fy_parser_set_default_document_state(), fy_reset_document_state()
	rc = fy_parser_set_default_document_state(fyp, NULL);
	if (rc != 0) {
		fprintf(stderr, "{\"error\": \"fy_parser_set_default_document_state failed\"}\n");
		fy_parser_destroy(fyp);
		free(input);
		return 1;
	}

	/* Iterate tokens using the low-level scanner API */
	// cref: fy_scan(), fy_scan_token_free()
	// cref: fy_token_get_type(), fy_token_start_mark(), fy_token_end_mark()
	// cref: fy_token_get_text()
	int error = 0;
	int saw_stream_end = 0;
	struct fy_token *fyt;
	while ((fyt = fy_scan(fyp)) != NULL) {
		enum fy_token_type type = fy_token_get_type(fyt);
		const char *type_str = token_type_name(type);
		if (type == FYTT_STREAM_END)
			saw_stream_end = 1;

		/* Get start/end marks */
		// cref: struct fy_mark { size_t input_pos; int line; int column; }
		const struct fy_mark *sm = fy_token_start_mark(fyt);
		const struct fy_mark *em = fy_token_end_mark(fyt);

		/* Get text content for tokens that carry values.
		   For TAG tokens, fy_token_get_text() returns the resolved URI
		   (e.g. "tag:yaml.org,2002:str" for "!!str"). We reconstruct the
		   raw shorthand form using handle + suffix to match yamalgam's
		   scanner output. For TAG_DIRECTIVE, we format "handle prefix"
		   with a space separator. */
		size_t text_len = 0;
		const char *text = NULL;
		char tag_buf[1024];
		if (type == FYTT_TAG) {
			size_t h_len = 0, s_len = 0;
			const char *handle = fy_tag_token_handle(fyt, &h_len);
			const char *suffix = fy_tag_token_suffix(fyt, &s_len);
			if (h_len == 0 && s_len > 0) {
				/* Verbatim tag: !<uri> */
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

		/* Emit JSON line */
		fprintf(stdout, "{\"type\":\"%s\",", type_str);
		if (text) {
			fprintf(stdout, "\"value\":\"");
			json_escape(stdout, text, text_len);
			fprintf(stdout, "\",");
		} else {
			fprintf(stdout, "\"value\":null,");
		}

		if (sm) {
			fprintf(stdout,
				"\"line\":%d,\"column\":%d,\"offset\":%zu,",
				sm->line, sm->column, sm->input_pos);
		} else {
			fprintf(stdout,
				"\"line\":null,\"column\":null,\"offset\":null,");
		}

		if (em) {
			fprintf(stdout,
				"\"end_line\":%d,\"end_column\":%d,\"end_offset\":%zu",
				em->line, em->column, em->input_pos);
		} else {
			fprintf(stdout,
				"\"end_line\":null,\"end_column\":null,\"end_offset\":null");
		}

		fprintf(stdout, "}\n");

		fy_scan_token_free(fyp, fyt);
	}

	/* fy_scan() returns NULL on both end-of-input and error.
	   If we never saw STREAM_END, something went wrong. */
	if (!saw_stream_end) {
		fprintf(stderr, "{\"error\":\"scan terminated without STREAM_END\"}\n");
		error = 1;
	}

	// cref: fy_parser_destroy()
	fy_parser_destroy(fyp);
	free(input);

	return error;
}
