const spec = {
  $schema: "https://vega.github.io/schema/vega-lite/v5.json",
  description: "A simple bar chart with embedded data.",
  width: 360,
  height: 360,
  selection: {
    grid: {
      type: "interval",
      bind: "scales",
    },
  },
  params: [
    { name: "xMin", value: 0 },
    { name: "xMax", value: 1000 },
    { name: "yMin", value: 0 },
    { name: "yMax", value: 1000 },
  ],
  data: {
    name: "table",
  },
  mark: "line",
  encoding: {
    x: {
      field: "x",
      type: "quantitative",
      axis: { grid: true },
      scale: { domain: [{ expr: "xMin" }, { expr: "xMax" }] },
    },
    y: {
      field: "y",
      type: "quantitative",
      axis: { grid: true },
      scale: { domain: [{ expr: "yMin" }, { expr: "yMax" }] },
    },
  },
};

console.log(spec);
vegaEmbed("#vis", spec, { defaultStyle: true })
  .then(function (result) {
    const view = result.view;

    // connect to simple echo server
    console.log("Connecting to /ws");
    const conn = new WebSocket("/ws");

    conn.onopen = function (event) {
      console.log("Connected to ws");
      conn.onmessage = function (event) {
        console.log(event.data);
        let to_render = JSON.parse(event.data);

        let xs = to_render.points.filter((p) => p.y !== null).map((p) => p.x);
        let ys = to_render.points.filter((p) => p.y !== null).map((p) => p.y);
        let min_x = Math.min(...xs);
        let max_x = Math.max(...xs);
        let center_x = (max_x + min_x) * 0.5;

        let min_y = Math.min(...ys);
        let max_y = Math.max(...ys);
        let center_y = (max_y + min_y) * 0.5;

        let radius = Math.max(max_x - min_x, max_y - min_y) / 2;
        view.signal("xMin", center_x - radius);
        view.signal("yMin", center_y - radius);
        view.signal("xMax", center_x + radius);
        view.signal("yMax", center_x + radius);

        // Use the Vega view api to insert data
        view.insert("table", to_render.points).run();
      };

      //// send some data into the echo socket every second
      //const interval = window.setInterval(function () {
      //  if (data.length) {
      //    conn.send(JSON.stringify(data.pop()));
      //  } else {
      //    clearInterval(interval);
      //  }
      //}, 1000);
    };
  })
  .catch(console.warn);
