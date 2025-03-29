const WASMTIME_20 = moment.utc('2024-04-20');
const DATE_FORMAT = 'MMMM D YYYY';

function releaseDate(version) {
  return WASMTIME_20.clone().add(version - 20, 'months');
}

function eolDate(version) {
  const release = releaseDate(version);
  if (version % 12 == 0)
    return release.add(2, 'year');
  return release.add(2, 'months');
}

function addReleases(table) {
  const today = moment.utc();
  const monthsSince20 = Math.floor(today.diff(WASMTIME_20, 'months'));
  const currentRelease = 20 + monthsSince20;

  // Add in some relevant releases, such as the current one, one future one, and
  // two past ones.
  let channels = [];
  channels.push(currentRelease + 1);
  channels.push(currentRelease);
  channels.push(currentRelease - 1);
  channels.push(currentRelease - 2);

  // Add in historical LTS channels. Start with the current release and go
  // backwards looking for two LTS channels.
  let lts = 0;
  let cur = currentRelease;
  for (let cur = currentRelease; cur > 20 && lts < 2; cur--) {
    if (cur % 12 == 0) {
      channels.push(cur);
      lts += 1;
    }
  }

  // Add in a future LTS channel starting with the release after the current
  // one.
  for (let cur = currentRelease + 1; ; cur++) {
    if (cur % 12 == 0) {
      channels.push(cur);
      break;
    }
  }

  // Deduplicate and sort everything.
  channels = [...new Set(channels)];
  channels.sort();

let mermaid = `
gantt
    tickInterval 12month
    title Release Schedule
    dateFormat  YYYY-MM
`;

  for (let channel of channels) {
    const row = document.createElement('tr');
    const date = releaseDate(channel);
    const eol = eolDate(channel);

    if (date <= today && today <= eol)
      row.style.fontWeight = 'bold';

    const version = document.createElement('td');
    version.innerText = channel + '.0.0';
    row.appendChild(version);

    const lts = document.createElement('td');
    if (channel % 12 == 0)
      lts.innerText = '✅';
    row.appendChild(lts);

    const branch = document.createElement('td');
    branch.innerText = date.clone().add(-15, 'days').format(DATE_FORMAT);
    row.appendChild(branch);

    const release = document.createElement('td');
    release.innerText = date.format(DATE_FORMAT);
    row.appendChild(release);

    const eolRow = document.createElement('td');
    eolRow.innerText = eol.format(DATE_FORMAT);
    row.appendChild(eolRow);

    dur = eol.diff(date, 'days')
    mermaid += `    ${channel}.0.0    :a1, ${date.format('YYYY-MM-DD')}, ${dur}d\n`;

    table.appendChild(row);
  }

  const gantt = document.createElement('pre');
  gantt.classList.add('mermaid');
  gantt.innerHTML = mermaid;
  document.querySelector('#version-table').appendChild(gantt);
}

const table = document.querySelector('#version-table table tbody');
if (table) {
  addReleases(table);
  mermaid.initialize({ theme: 'dark' });
}
