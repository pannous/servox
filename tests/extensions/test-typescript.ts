const hello: string = "Hello from external TypeScript file!";
console.log(hello);
document.getElementById('test0').innerHTML =
    `<span class="result">✓ PASS:</span> ${hello}`;