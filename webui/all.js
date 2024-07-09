
        let apiBaseUrl = 'http://127.0.0.1:5010';

        const urlParams = new URLSearchParams(window.location.search);
        apiBaseUrl = urlParams.get('apibaseUrl')
        fetchLibrary();


        async function fetchLibrary() {
            try {
                const response = await fetch(`${apiBaseUrl}/show_library`);
                if (response.ok) {
                    const results = await response.json();
                    displaySearchResults(results);
                } else {
                    console.error('Failed to fetch library results:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        async function fetchFile(filePath) {
            try {
                const response = await fetch(`${apiBaseUrl}/getfile?file_path=${encodeURIComponent(filePath)}`);
                if (response.ok) {
                    return response.blob();
                } else {
                    console.error('Failed to fetch file:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
            return null;
        }

        function displaySearchResults(results) {
            displaySongResults(results.songs);
            displayAlbumResults(results.albums);
            displayArtistResults(results.artists);
        }

        async function displaySongResults(songs) {
            const songResults = document.getElementById('song-results');
            songResults.innerHTML = '';
            for (const song of songs) {
                const songItem = document.createElement('div');
                songItem.className = 'col-12 song-item';
                songItem.onclick = () => {
                    playSong(song);
                };

                const songInfo = document.createElement('div');
                songInfo.className = 'song-info';

                const songArt = document.createElement('img');
                songArt.className = 'song-art';
                const songArtBlob = await fetchFile(song.song_art_path);
                if (songArtBlob) {
                    const songArtUrl = URL.createObjectURL(songArtBlob);
                    songArt.src = songArtUrl;
                } else {
                    songArt.src = 'default_cover.png';
                }
                songArt.alt = song.name;

                const songDetails = document.createElement('div');
                songDetails.className = 'details';

                const songTitle = document.createElement('div');
                songTitle.className = 'title';
                songTitle.textContent = song.name;

                const songArtist = document.createElement('div');
                songArtist.className = 'artist';
                song.artists.forEach(artist => {
                    const artistLink = document.createElement('a');
                    artistLink.href = '#';
                    artistLink.textContent = artist.name;
                    artistLink.style.color = '#007bff';
                    artistLink.style.marginRight = '5px';
                    artistLink.onclick = (event) => {
                        event.stopPropagation();
                        window.parent.postMessage({ action: 'navigateTo', url: `artist.html?uuid=${artist.uuid}` }, '*');
                    };
                    songArtist.appendChild(artistLink);
                });

                const albumName = document.createElement('a');
                albumName.className = 'album-name';
                albumName.textContent = song.album.name;
                albumName.href = `album.html?uuid=${song.album.uuid}`;
                albumName.onclick = (event) => {
                    event.stopPropagation();
                    window.parent.postMessage({ action: 'navigateTo', url: `album.html?uuid=${song.album.uuid}` }, '*');
                };

                songDetails.appendChild(songTitle);
                songDetails.appendChild(songArtist);
                songDetails.appendChild(albumName);
                songInfo.appendChild(songArt);
                songInfo.appendChild(songDetails);

                const songActions = document.createElement('div');
                songActions.className = 'actions';

                const likedIcon = document.createElement('span');
                likedIcon.className = song.is_liked ? 'text-danger liked' : 'liked';
                likedIcon.innerHTML = song.is_liked ? '&#9829;' : '&#9825;';
                likedIcon.addEventListener('click', (event) => {
                    event.stopPropagation();
                    toggleSongLike(song.uuid, likedIcon);
                });
                songActions.appendChild(likedIcon);

                songItem.appendChild(songInfo);
                songItem.appendChild(songActions);
                songResults.appendChild(songItem);
            }
        }

        async function displayAlbumResults(albums) {
            const albumResults = document.getElementById('album-results');
            albumResults.innerHTML = '';
            for (const album of albums) {
                const albumItem = document.createElement('div');
                albumItem.className = 'col-md-3 col-sm-4 col-6 album-item';
                albumItem.onclick = () => {
                    window.parent.postMessage({ action: 'navigateTo', url: `album.html?uuid=${album.uuid}` }, '*');
                };

                const card = document.createElement('div');
                card.className = 'card bg-dark text-white';
                card.style.cursor = 'pointer';

                const albumArt = document.createElement('img');
                albumArt.className = 'album-art card-img-top';
                const albumArtBlob = await fetchFile(album.album_art_path);
                if (albumArtBlob) {
                    const albumArtUrl = URL.createObjectURL(albumArtBlob);
                    albumArt.src = albumArtUrl;
                } else {
                    albumArt.src = 'default_album.png';
                }
                albumArt.alt = album.name;

                const cardBody = document.createElement('div');
                cardBody.className = 'card-body';

                const albumName = document.createElement('h5');
                albumName.className = 'card-title';
                albumName.textContent = album.name;

                const albumYear = document.createElement('p');
                albumYear.className = 'card-text';
                albumYear.textContent = album.year;

                cardBody.appendChild(albumName);
                cardBody.appendChild(albumYear);
                card.appendChild(albumArt);
                card.appendChild(cardBody);
                albumItem.appendChild(card);
                albumResults.appendChild(albumItem);
            }
        }

        async function displayArtistResults(artists) {
            const artistResults = document.getElementById('artist-results');
            artistResults.innerHTML = '';
            for (const artist of artists) {
                const artistItem = document.createElement('div');
                artistItem.className = 'col-md-3 col-sm-4 col-6 artist-item';
                artistItem.onclick = () => {
                    window.parent.postMessage({ action: 'navigateTo', url: `artist.html?uuid=${artist.uuid}` }, '*');
                };

                const card = document.createElement('div');
                card.className = 'card bg-dark text-white';
                card.style.cursor = 'pointer';

                const artistArt = document.createElement('img');
                artistArt.className = 'artist-art card-img-top';
                const artistArtBlob = await fetchFile(artist.artist_art_path);
                if (artistArtBlob) {
                    const artistArtUrl = URL.createObjectURL(artistArtBlob);
                    artistArt.src = artistArtUrl;
                } else {
                    artistArt.src = 'default_artist.png';
                }
                artistArt.alt = artist.name;

                const cardBody = document.createElement('div');
                cardBody.className = 'card-body';

                const artistName = document.createElement('h5');
                artistName.className = 'card-title';
                artistName.textContent = artist.name;

                cardBody.appendChild(artistName);
                card.appendChild(artistArt);
                card.appendChild(cardBody);
                artistItem.appendChild(card);
                artistResults.appendChild(artistItem);
            }
        }

        async function playSong(song) {
            try {
                console.log('Playing song:', song);
                if (!song.file_path) {
                    console.error('Error: song.file_path is undefined');
                    return;
                }

                const songArtBlob = await fetchFile(song.song_art_path);
                let songArtUrl = 'default_cover.png';
                if (songArtBlob) {
                    songArtUrl = URL.createObjectURL(songArtBlob);
                }

                window.parent.postMessage({
                    action: 'playSong',
                    song: {
                        filePath: song.file_path,
                        art: songArtUrl,
                        title: song.name,
                        artists: song.artists.map(artist => ({ name: artist.name, uuid: artist.uuid })),
                        album: song.album.name,
                        albumUUID: song.album.uuid,
                        isLiked: song.is_liked,
                        uuid: song.uuid
                    }
                }, '*');

                currentSong = song;
            } catch (error) {
                console.error('Error:', error);
            }
        }

        async function toggleSongLike(uuid, likedIcon) {
            try {
                const response = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
                if (response.ok) {
                    const song = await response.json();
                    let likeResponse;
                    if (song.is_liked) {
                        likeResponse = await fetch(`${apiBaseUrl}/unlike_song/${uuid}`);
                    } else {
                        likeResponse = await fetch(`${apiBaseUrl}/like_song/${uuid}`);
                    }

                    if (likeResponse.ok) {
                        const updatedSongResponse = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
                        if (updatedSongResponse.ok) {
                            const updatedSong = await updatedSongResponse.json();
                            likedIcon.className = updatedSong.is_liked ? 'text-danger liked' : 'liked';
                            likedIcon.innerHTML = updatedSong.is_liked ? '&#9829;' : '&#9825;';
                        }
                    } else {
                        console.error('Failed to toggle like status:', likeResponse.statusText);
                    }
                } else {
                    console.error('Failed to fetch song details:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        function updateSongListIcon(uuid, isLiked) {
            const songItem = document.getElementById(`song-${uuid}`);
            if (songItem) {
                const likedIcon = songItem.querySelector('.actions span');
                likedIcon.className = isLiked ? 'text-danger liked' : 'liked';
                likedIcon.innerHTML = isLiked ? '&#9829;' : '&#9825;';
            }
        }
    
