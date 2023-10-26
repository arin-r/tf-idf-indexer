console.log("hello!");

fetch("/api/search", {
  method: "POST",
  mode: "cors",
  cache: "no-cache",
  credentials: "same-origin",
  headers: {
    "Content-Type": "application/json",
  },
  redirect: "follow",
  referrerPolicy: "no-referrer",

  body: JSON.stringify('{ "name": "John Doe" }'),
})
  .then((response) => {
    console.log(response);
    return response.json();
  })
  .then((data) => {
    console.log(data);
  });
