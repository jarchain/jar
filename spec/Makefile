BOOK_BIN := .lake/build/bin/jarbook

.PHONY: book clean test

book: $(BOOK_BIN)
	$(BOOK_BIN)

$(BOOK_BIN):
	lake build jarbook

test:
	lake build jartest
	.lake/build/bin/jartest

clean:
	rm -rf _out
