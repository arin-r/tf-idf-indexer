console.log("hello!");

fetch("/api/search", {
  method: "POST",
  mode: "cors",
  cache: "no-cache",
  credentials: "same-origin",
  headers: {
    "Content-Type": "text/plain",
  },
  redirect: "follow",
  referrerPolicy: "no-referrer",

  // body: JSON.stringify('{ "query": "bind texture to buffer" }'),
  body: "bind texture to buffer",
})
  .then((response) => {
    console.log(response);
    return response.json();
  })
  .then((data) => {
    console.log(data);
  });
