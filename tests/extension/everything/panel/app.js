synt.on("message", (message) => {
  document.getElementById("note").value = JSON.stringify(message);
});

synt.call({ hello: "editor" });
