(() => {
  const formatter = new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
  for (const time of document.querySelectorAll("time[data-local-date]")) {
    const date = new Date(time.dateTime);
    if (!Number.isNaN(date.getTime())) time.textContent = formatter.format(date);
  }
})();
