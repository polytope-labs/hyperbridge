# Runtime controls on the Overview

`Operator` owns `togglePause` and `stopFiller` (they call `/api/pause`, `/api/resume`, `/api/stop`
through `useAction`) and passes them with `pending` to `OperatorOverview` as `runtime`. The
overview renders them inline between the metrics strip and the balances: a status dot and copy on
the left, Pause new fills / Resume filling (primary) and Stop filler (destructive styling) on the
right. There is no runtime sheet any more.
