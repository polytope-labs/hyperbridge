# 2026-09-05 — Order history pages server-side by order, not by event

Chosen: a dedicated history endpoint groups events per order in SQL and pages over distinct orders,
joining bids by commitment on the way out. The maintainer asked for pagination and for bids to be
folded into the order rows.

Alternatives rejected: paging the raw event feed by id (the old `before` cursor) and grouping in
the browser gives pages of uneven order counts and can split an order across pages; keeping a
separate bids table repeats the commitment the order row already shows. The live stream now
triggers a re-read of the current page rather than a client-side merge, because a merged row
cannot know whether it still belongs on the page being viewed.
